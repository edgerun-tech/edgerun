//! Zero-dependency derived database for rebuildable Edgerun projections.
//!
//! This crate is not an internal wire protocol and is not authoritative state.
//! It stores typed, derived rows behind a small append-only log. Authoritative
//! state still comes from the event log.

#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt;

use edgerun_wire::{
    DerivedDbAdminAuditRecord as AdminAuditRecordWire, DerivedDbKeyRecord as KeyRecordWire,
    DerivedDbMetaRecord as MetaRecordWire, DerivedDbObjectIndexRecord as ObjectIndexRecordWire,
};

const MAGIC: &[u8; 8] = b"ERDB0001";
const FORMAT_VERSION: u16 = 2;
const SCHEMA_VERSION: u16 = 1;
const HEADER_FLAGS: u32 = 0;
const FILE_HEADER_LEN: u64 = 20;
const RECORD_META: u8 = 1;
const RECORD_OBJECT_INDEX: u8 = 2;
const RECORD_ADMIN_AUDIT: u8 = 3;
const RECORD_DELETE_META: u8 = 4;
const RECORD_DELETE_OBJECT_INDEX: u8 = 5;
const RECORD_HEADER_LEN: u64 = 9;
const MAX_RECORD_PAYLOAD_LEN: usize = 16 * 1024 * 1024;

/// Storage plugged into the internal derived database.
pub trait DbStorage {
    fn len(&mut self) -> Result<u64, DbError>;
    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), DbError>;
    fn append(&mut self, bytes: &[u8]) -> Result<u64, DbError>;
    fn truncate(&mut self, _len: u64) -> Result<(), DbError> {
        Err(DbError::Storage(
            "storage does not support truncation".into(),
        ))
    }
    fn sync(&mut self) -> Result<(), DbError> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbError {
    Storage(String),
    Decode(String),
    InvalidArgument(String),
    NeedsRebuild(String),
    Sql(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(message) => write!(f, "storage error: {message}"),
            Self::Decode(message) => write!(f, "decode error: {message}"),
            Self::InvalidArgument(message) => write!(f, "invalid argument: {message}"),
            Self::NeedsRebuild(message) => write!(f, "needs rebuild: {message}"),
            Self::Sql(message) => write!(f, "sql error: {message}"),
        }
    }
}

impl core::error::Error for DbError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeMetaRow {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub updated_at: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectIndexRow<'a> {
    pub object_id: &'a [u8],
    pub stream_id: &'a [u8],
    pub seq: u64,
    pub representation_id: &'a [u8],
    pub content_type: Option<&'a str>,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectIndexRowOwned {
    pub object_id: Vec<u8>,
    pub stream_id: Vec<u8>,
    pub seq: u64,
    pub representation_id: Vec<u8>,
    pub content_type: Option<String>,
    pub updated_at: i64,
}

impl ObjectIndexRowOwned {
    fn as_borrowed(&self) -> ObjectIndexRow<'_> {
        ObjectIndexRow {
            object_id: &self.object_id,
            stream_id: &self.stream_id,
            seq: self.seq,
            representation_id: &self.representation_id,
            content_type: self.content_type.as_deref(),
            updated_at: self.updated_at,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdminAuditRow<'a> {
    pub event_time: i64,
    pub subject: &'a str,
    pub action: &'a str,
    pub outcome: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdminAuditRowOwned {
    pub id: u64,
    pub event_time: i64,
    pub subject: String,
    pub action: String,
    pub outcome: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SqlValue {
    Null,
    Integer(i64),
    Bytes(Vec<u8>),
    Text(String),
}

impl SqlValue {
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(value) => Some(value),
            Self::Text(value) => Some(value.as_bytes()),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<SqlValue>>,
    pub rows_affected: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqlAccessPath {
    FullScan,
    RuntimeMetaKey,
    ObjectIndexObjectId,
    ObjectIndexStream,
    ObjectIndexStreamSeq,
    AdminAuditId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlExplain {
    pub table: &'static str,
    pub access_path: SqlAccessPath,
    pub ordered: bool,
    pub limit: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqlColumnType {
    Integer,
    Bytes,
    Text,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SqlColumnSchema {
    pub name: &'static str,
    pub ty: SqlColumnType,
    pub nullable: bool,
    pub primary_key: bool,
    pub mutable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SqlTableSchema {
    pub name: &'static str,
    pub columns: &'static [SqlColumnSchema],
    pub append_only: bool,
}

const RUNTIME_META_COLUMNS: &[SqlColumnSchema] = &[
    SqlColumnSchema {
        name: "key",
        ty: SqlColumnType::Bytes,
        nullable: false,
        primary_key: true,
        mutable: false,
    },
    SqlColumnSchema {
        name: "value",
        ty: SqlColumnType::Bytes,
        nullable: false,
        primary_key: false,
        mutable: true,
    },
    SqlColumnSchema {
        name: "updated_at",
        ty: SqlColumnType::Integer,
        nullable: false,
        primary_key: false,
        mutable: true,
    },
];

const DERIVED_OBJECT_INDEX_COLUMNS: &[SqlColumnSchema] = &[
    SqlColumnSchema {
        name: "object_id",
        ty: SqlColumnType::Bytes,
        nullable: false,
        primary_key: true,
        mutable: false,
    },
    SqlColumnSchema {
        name: "stream_id",
        ty: SqlColumnType::Bytes,
        nullable: false,
        primary_key: false,
        mutable: true,
    },
    SqlColumnSchema {
        name: "seq",
        ty: SqlColumnType::Integer,
        nullable: false,
        primary_key: false,
        mutable: true,
    },
    SqlColumnSchema {
        name: "representation_id",
        ty: SqlColumnType::Bytes,
        nullable: false,
        primary_key: false,
        mutable: true,
    },
    SqlColumnSchema {
        name: "content_type",
        ty: SqlColumnType::Text,
        nullable: true,
        primary_key: false,
        mutable: true,
    },
    SqlColumnSchema {
        name: "updated_at",
        ty: SqlColumnType::Integer,
        nullable: false,
        primary_key: false,
        mutable: true,
    },
];

const ADMIN_AUDIT_COLUMNS: &[SqlColumnSchema] = &[
    SqlColumnSchema {
        name: "id",
        ty: SqlColumnType::Integer,
        nullable: false,
        primary_key: true,
        mutable: false,
    },
    SqlColumnSchema {
        name: "event_time",
        ty: SqlColumnType::Integer,
        nullable: false,
        primary_key: false,
        mutable: false,
    },
    SqlColumnSchema {
        name: "subject",
        ty: SqlColumnType::Text,
        nullable: false,
        primary_key: false,
        mutable: false,
    },
    SqlColumnSchema {
        name: "action",
        ty: SqlColumnType::Text,
        nullable: false,
        primary_key: false,
        mutable: false,
    },
    SqlColumnSchema {
        name: "outcome",
        ty: SqlColumnType::Text,
        nullable: false,
        primary_key: false,
        mutable: false,
    },
];

const SQL_SCHEMA: &[SqlTableSchema] = &[
    SqlTableSchema {
        name: "runtime_meta",
        columns: RUNTIME_META_COLUMNS,
        append_only: false,
    },
    SqlTableSchema {
        name: "derived_object_index",
        columns: DERIVED_OBJECT_INDEX_COLUMNS,
        append_only: false,
    },
    SqlTableSchema {
        name: "admin_audit",
        columns: ADMIN_AUDIT_COLUMNS,
        append_only: true,
    },
];

pub fn sql_schema() -> &'static [SqlTableSchema] {
    SQL_SCHEMA
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DbBatch {
    records: Vec<PendingRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PendingRecord {
    PutMeta(RuntimeMetaRow),
    DeleteMeta(Vec<u8>),
    UpsertObjectIndex(ObjectIndexRowOwned),
    DeleteObjectIndex(Vec<u8>),
    AppendAdminAudit(AdminAuditDraftOwned),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AdminAuditDraftOwned {
    event_time: i64,
    subject: String,
    action: String,
    outcome: String,
}

impl DbBatch {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn put_meta(
        &mut self,
        key: &[u8],
        value: &[u8],
        updated_at: i64,
    ) -> Result<&mut Self, DbError> {
        require_non_empty(key, "runtime_meta.key")?;
        self.records.push(PendingRecord::PutMeta(RuntimeMetaRow {
            key: key.to_vec(),
            value: value.to_vec(),
            updated_at,
        }));
        Ok(self)
    }

    pub fn delete_meta(&mut self, key: &[u8]) -> Result<&mut Self, DbError> {
        require_non_empty(key, "runtime_meta.key")?;
        self.records.push(PendingRecord::DeleteMeta(key.to_vec()));
        Ok(self)
    }

    pub fn upsert_object_index(&mut self, row: ObjectIndexRow<'_>) -> Result<&mut Self, DbError> {
        validate_object_index_row(row)?;
        self.records
            .push(PendingRecord::UpsertObjectIndex(row.to_owned()));
        Ok(self)
    }

    pub fn delete_object_index(&mut self, object_id: &[u8]) -> Result<&mut Self, DbError> {
        require_non_empty(object_id, "derived_object_index.object_id")?;
        self.records
            .push(PendingRecord::DeleteObjectIndex(object_id.to_vec()));
        Ok(self)
    }

    pub fn append_admin_audit(&mut self, row: AdminAuditRow<'_>) -> Result<&mut Self, DbError> {
        self.records
            .push(PendingRecord::AppendAdminAudit(row.to_owned_draft()));
        Ok(self)
    }
}

/// Append-only derived database over caller-provided storage.
pub struct DerivedDb<S> {
    storage: S,
    runtime_meta: BTreeMap<Vec<u8>, RuntimeMetaRow>,
    object_index: BTreeMap<Vec<u8>, ObjectIndexRowOwned>,
    object_index_by_stream: BTreeMap<Vec<u8>, Vec<Vec<u8>>>,
    object_index_by_stream_seq: BTreeMap<(Vec<u8>, u64), Vec<Vec<u8>>>,
    admin_audit: Vec<AdminAuditRowOwned>,
    admin_audit_by_id: BTreeMap<u64, usize>,
    next_admin_audit_id: u64,
}

impl<S: DbStorage> DerivedDb<S> {
    pub fn open(mut storage: S) -> Result<Self, DbError> {
        Self::open_inner(storage, false)
    }

    /// Opens a database and truncates only an incomplete final append record.
    ///
    /// This is intended for power-loss recovery after the storage layer
    /// persisted a prefix of the last append. Complete records with checksum
    /// mismatches, unknown record kinds, bad headers, or incompatible schema
    /// versions still fail loudly.
    pub fn open_repairing_tail(storage: S) -> Result<Self, DbError> {
        Self::open_inner(storage, true)
    }

    fn open_inner(mut storage: S, repair_tail: bool) -> Result<Self, DbError> {
        let len = storage.len()?;
        if len == 0 {
            storage.append(&file_header())?;
            storage.sync()?;
        } else {
            validate_file_header(&mut storage, len)?;
        }

        let mut db = Self {
            storage,
            runtime_meta: BTreeMap::new(),
            object_index: BTreeMap::new(),
            object_index_by_stream: BTreeMap::new(),
            object_index_by_stream_seq: BTreeMap::new(),
            admin_audit: Vec::new(),
            admin_audit_by_id: BTreeMap::new(),
            next_admin_audit_id: 1,
        };
        db.replay(repair_tail)?;
        Ok(db)
    }

    pub fn into_storage(self) -> S {
        self.storage
    }

    pub fn put_meta(&mut self, key: &[u8], value: &[u8], updated_at: i64) -> Result<(), DbError> {
        require_non_empty(key, "runtime_meta.key")?;
        let payload = encode_meta_payload(key, value, updated_at)?;
        self.append_record(RECORD_META, &payload)?;
        self.apply_meta(key.to_vec(), value.to_vec(), updated_at);
        Ok(())
    }

    pub fn get_meta(&self, key: &[u8]) -> Option<&RuntimeMetaRow> {
        self.runtime_meta.get(key)
    }

    pub fn delete_meta(&mut self, key: &[u8]) -> Result<bool, DbError> {
        require_non_empty(key, "runtime_meta.key")?;
        let payload = encode_key_payload(key)?;
        self.append_record(RECORD_DELETE_META, &payload)?;
        Ok(self.runtime_meta.remove(key).is_some())
    }

    pub fn meta_rows(&self) -> impl Iterator<Item = &RuntimeMetaRow> {
        self.runtime_meta.values()
    }

    pub fn meta_rows_with_prefix<'a>(
        &'a self,
        prefix: &'a [u8],
    ) -> impl Iterator<Item = &'a RuntimeMetaRow> + 'a {
        self.runtime_meta
            .values()
            .filter(move |row| row.key.starts_with(prefix))
    }

    pub fn upsert_object_index(&mut self, row: ObjectIndexRow<'_>) -> Result<(), DbError> {
        validate_object_index_row(row)?;

        let payload = encode_object_index_payload(row)?;
        self.append_record(RECORD_OBJECT_INDEX, &payload)?;
        self.apply_object_index(row);
        Ok(())
    }

    pub fn get_object_index(&self, object_id: &[u8]) -> Option<&ObjectIndexRowOwned> {
        self.object_index.get(object_id)
    }

    pub fn delete_object_index(&mut self, object_id: &[u8]) -> Result<bool, DbError> {
        require_non_empty(object_id, "derived_object_index.object_id")?;
        let payload = encode_key_payload(object_id)?;
        self.append_record(RECORD_DELETE_OBJECT_INDEX, &payload)?;
        Ok(self.remove_object_index(object_id).is_some())
    }

    pub fn object_index_rows(&self) -> impl Iterator<Item = &ObjectIndexRowOwned> {
        self.object_index.values()
    }

    pub fn object_index_for_stream<'a>(
        &'a self,
        stream_id: &'a [u8],
    ) -> impl Iterator<Item = &'a ObjectIndexRowOwned> + 'a {
        self.object_index_by_stream
            .get(stream_id)
            .into_iter()
            .flat_map(move |object_ids| object_ids.iter())
            .filter_map(move |object_id| self.object_index.get(object_id))
    }

    pub fn object_index_for_stream_seq<'a>(
        &'a self,
        stream_id: &'a [u8],
        seq: u64,
    ) -> impl Iterator<Item = &'a ObjectIndexRowOwned> + 'a {
        self.object_index_by_stream_seq
            .get(&(stream_id.to_vec(), seq))
            .into_iter()
            .flat_map(move |object_ids| object_ids.iter())
            .filter_map(move |object_id| self.object_index.get(object_id))
    }

    pub fn append_admin_audit(&mut self, row: AdminAuditRow<'_>) -> Result<u64, DbError> {
        let id = self.next_admin_audit_id;
        let payload =
            encode_admin_audit_payload(id, row.event_time, row.subject, row.action, row.outcome)?;
        self.append_record(RECORD_ADMIN_AUDIT, &payload)?;
        self.apply_admin_audit(id, row);
        Ok(id)
    }

    pub fn admin_audit(&self) -> &[AdminAuditRowOwned] {
        &self.admin_audit
    }

    pub fn next_admin_audit_id(&self) -> u64 {
        self.next_admin_audit_id
    }

    pub fn apply_batch(&mut self, batch: DbBatch) -> Result<(), DbError> {
        if batch.records.is_empty() {
            return Ok(());
        }

        let mut encoded = Vec::new();
        let mut next_admin_audit_id = self.next_admin_audit_id;
        for record in &batch.records {
            let (kind, payload) = encode_pending_record(record, &mut next_admin_audit_id)?;
            append_encoded_record(&mut encoded, kind, &payload)?;
        }
        self.storage.append(&encoded)?;
        self.storage.sync()?;

        for record in batch.records {
            self.apply_pending_record(record);
        }
        Ok(())
    }

    pub fn compact_into<T: DbStorage>(&self, storage: T) -> Result<DerivedDb<T>, DbError> {
        let mut compacted = DerivedDb::open(storage)?;
        for row in self.runtime_meta.values() {
            compacted.put_meta(&row.key, &row.value, row.updated_at)?;
        }
        for row in self.object_index.values() {
            compacted.upsert_object_index(ObjectIndexRow {
                object_id: &row.object_id,
                stream_id: &row.stream_id,
                seq: row.seq,
                representation_id: &row.representation_id,
                content_type: row.content_type.as_deref(),
                updated_at: row.updated_at,
            })?;
        }
        for row in &self.admin_audit {
            compacted.append_admin_audit(AdminAuditRow {
                event_time: row.event_time,
                subject: &row.subject,
                action: &row.action,
                outcome: &row.outcome,
            })?;
        }
        Ok(compacted)
    }

    /// Executes the crate's constrained SQL facade.
    ///
    /// Supported statements are fixed-table `SELECT`, `INSERT INTO`,
    /// primary-key `UPDATE`, and primary-key `DELETE`. `WHERE` supports
    /// equality and integer range predicates joined by `AND`; `SELECT` additionally supports
    /// `COUNT(*)`, single-column `ORDER BY`, and `LIMIT`.
    pub fn execute_sql(&mut self, sql: &str, params: &[SqlValue]) -> Result<SqlResult, DbError> {
        let statement = parse_sql(sql)?;
        let explain = explain_statement(&statement);
        match statement {
            SqlStatement::Select {
                columns,
                table,
                filters,
                order_by,
                limit,
            } => self.execute_select(
                columns,
                table,
                &filters,
                explain.access_path,
                order_by.as_ref(),
                limit,
                params,
            ),
            statement => self.execute_write_statement(&statement, params),
        }
    }

    pub fn execute_sql_batch(
        &mut self,
        statements: &[(&str, &[SqlValue])],
    ) -> Result<SqlResult, DbError> {
        let mut runtime_meta = self.runtime_meta.clone();
        let mut object_index = self.object_index.clone();
        let mut records = Vec::new();
        let mut rows_affected = 0usize;

        for (sql, params) in statements {
            let statement = parse_sql(sql)?;
            let (record, affected) =
                stage_sql_write(&statement, params, &mut runtime_meta, &mut object_index)?;
            if let Some(record) = record {
                records.push(record);
            }
            rows_affected = rows_affected.saturating_add(affected);
        }

        self.apply_batch(DbBatch { records })?;
        Ok(sql_write_result(rows_affected))
    }

    pub fn explain_sql(&self, sql: &str) -> Result<SqlExplain, DbError> {
        let statement = parse_sql(sql)?;
        Ok(explain_statement(&statement))
    }

    pub fn sql_schema(&self) -> &'static [SqlTableSchema] {
        sql_schema()
    }

    fn execute_select(
        &self,
        columns: Vec<String>,
        table: SqlTable,
        filters: &[SqlFilter],
        access_path: SqlAccessPath,
        order_by: Option<&SqlOrder>,
        limit: Option<usize>,
        params: &[SqlValue],
    ) -> Result<SqlResult, DbError> {
        if is_count_star(&columns) {
            return self.execute_count(table, filters, access_path, params);
        }

        let selected = expand_select_columns(table, columns)?;
        if let Some(order_by) = order_by {
            validate_column(table, &order_by.column)?;
        }
        let mut rows = Vec::new();
        match table {
            SqlTable::RuntimeMeta => {
                for row in self.runtime_meta_candidates(filters, access_path, params)? {
                    if matches_runtime_meta_filters(row, filters, params)? {
                        rows.push((
                            order_by
                                .map(|order_by| runtime_meta_column_value(row, &order_by.column))
                                .transpose()?,
                            project_runtime_meta_row(row, &selected)?,
                        ));
                    }
                }
            }
            SqlTable::DerivedObjectIndex => {
                for row in self.object_index_candidates(filters, access_path, params)? {
                    if matches_object_index_filters(row, filters, params)? {
                        rows.push((
                            order_by
                                .map(|order_by| object_index_column_value(row, &order_by.column))
                                .transpose()?,
                            project_object_index_row(row, &selected)?,
                        ));
                    }
                }
            }
            SqlTable::AdminAudit => {
                for row in self.admin_audit_candidates(filters, access_path, params)? {
                    if matches_admin_audit_filters(row, filters, params)? {
                        rows.push((
                            order_by
                                .map(|order_by| admin_audit_column_value(row, &order_by.column))
                                .transpose()?,
                            project_admin_audit_row(row, &selected)?,
                        ));
                    }
                }
            }
        }
        if let Some(order_by) = order_by {
            rows.sort_by(|(left, _), (right, _)| {
                let ordering = compare_sql_values(left.as_ref().unwrap(), right.as_ref().unwrap());
                if order_by.descending {
                    ordering.reverse()
                } else {
                    ordering
                }
            });
        }
        let mut rows: Vec<Vec<SqlValue>> = rows.into_iter().map(|(_, row)| row).collect();
        if let Some(limit) = limit {
            rows.truncate(limit);
        }
        Ok(SqlResult {
            columns: selected,
            rows,
            rows_affected: 0,
        })
    }

    fn execute_count(
        &self,
        table: SqlTable,
        filters: &[SqlFilter],
        access_path: SqlAccessPath,
        params: &[SqlValue],
    ) -> Result<SqlResult, DbError> {
        let count = match table {
            SqlTable::RuntimeMeta => self
                .runtime_meta_candidates(filters, access_path, params)?
                .into_iter()
                .try_fold(0usize, |count, row| {
                    Ok(count + usize::from(matches_runtime_meta_filters(row, filters, params)?))
                })?,
            SqlTable::DerivedObjectIndex => self
                .object_index_candidates(filters, access_path, params)?
                .into_iter()
                .try_fold(0usize, |count, row| {
                    Ok(count + usize::from(matches_object_index_filters(row, filters, params)?))
                })?,
            SqlTable::AdminAudit => self
                .admin_audit_candidates(filters, access_path, params)?
                .into_iter()
                .try_fold(0usize, |count, row| {
                    Ok(count + usize::from(matches_admin_audit_filters(row, filters, params)?))
                })?,
        };

        Ok(SqlResult {
            columns: alloc::vec!["count".to_string()],
            rows: alloc::vec![alloc::vec![SqlValue::Integer(count as i64)]],
            rows_affected: 0,
        })
    }

    fn runtime_meta_candidates(
        &self,
        filters: &[SqlFilter],
        access_path: SqlAccessPath,
        params: &[SqlValue],
    ) -> Result<Vec<&RuntimeMetaRow>, DbError> {
        match access_path {
            SqlAccessPath::RuntimeMetaKey => {
                let Some(key) = filter_bytes_for_column(filters, params, "key")? else {
                    return Ok(Vec::new());
                };
                Ok(self.runtime_meta.get(key).into_iter().collect())
            }
            _ => Ok(self.runtime_meta.values().collect()),
        }
    }

    fn object_index_candidates(
        &self,
        filters: &[SqlFilter],
        access_path: SqlAccessPath,
        params: &[SqlValue],
    ) -> Result<Vec<&ObjectIndexRowOwned>, DbError> {
        match access_path {
            SqlAccessPath::ObjectIndexObjectId => {
                let Some(object_id) = filter_bytes_for_column(filters, params, "object_id")? else {
                    return Ok(Vec::new());
                };
                Ok(self.object_index.get(object_id).into_iter().collect())
            }
            SqlAccessPath::ObjectIndexStreamSeq => {
                let Some(stream_id) = filter_bytes_for_column(filters, params, "stream_id")? else {
                    return Ok(Vec::new());
                };
                let Some(seq) = filter_u64_for_column(filters, params, "seq")? else {
                    return Ok(Vec::new());
                };
                Ok(self
                    .object_index_by_stream_seq
                    .get(&(stream_id.to_vec(), seq))
                    .into_iter()
                    .flat_map(|object_ids| object_ids.iter())
                    .filter_map(|object_id| self.object_index.get(object_id))
                    .collect())
            }
            SqlAccessPath::ObjectIndexStream => {
                let Some(stream_id) = filter_bytes_for_column(filters, params, "stream_id")? else {
                    return Ok(Vec::new());
                };
                Ok(self
                    .object_index_by_stream
                    .get(stream_id)
                    .into_iter()
                    .flat_map(|object_ids| object_ids.iter())
                    .filter_map(|object_id| self.object_index.get(object_id))
                    .collect())
            }
            _ => Ok(self.object_index.values().collect()),
        }
    }

    fn admin_audit_candidates(
        &self,
        filters: &[SqlFilter],
        access_path: SqlAccessPath,
        params: &[SqlValue],
    ) -> Result<Vec<&AdminAuditRowOwned>, DbError> {
        match access_path {
            SqlAccessPath::AdminAuditId => {
                let Some(id) = filter_u64_for_column(filters, params, "id")? else {
                    return Ok(Vec::new());
                };
                Ok(self
                    .admin_audit_by_id
                    .get(&id)
                    .and_then(|idx| self.admin_audit.get(*idx))
                    .into_iter()
                    .collect())
            }
            _ => Ok(self.admin_audit.iter().collect()),
        }
    }

    fn execute_write_statement(
        &mut self,
        statement: &SqlStatement,
        params: &[SqlValue],
    ) -> Result<SqlResult, DbError> {
        let mut runtime_meta = self.runtime_meta.clone();
        let mut object_index = self.object_index.clone();
        let (record, rows_affected) =
            stage_sql_write(statement, params, &mut runtime_meta, &mut object_index)?;
        let records = record.into_iter().collect();
        self.apply_batch(DbBatch { records })?;
        Ok(sql_write_result(rows_affected))
    }

    fn append_record(&mut self, kind: u8, payload: &[u8]) -> Result<(), DbError> {
        let mut record = Vec::new();
        append_encoded_record(&mut record, kind, payload)?;
        self.storage.append(&record)?;
        self.storage.sync()
    }

    fn replay(&mut self, repair_tail: bool) -> Result<(), DbError> {
        let len = self.storage.len()?;
        let mut offset = FILE_HEADER_LEN;
        while offset < len {
            let record_start = offset;
            if len.saturating_sub(offset) < RECORD_HEADER_LEN {
                if repair_tail {
                    self.storage.truncate(record_start)?;
                    self.storage.sync()?;
                    break;
                }
                return Err(DbError::Decode(format!(
                    "truncated derived db record header at offset {offset}"
                )));
            }

            let mut header = [0u8; RECORD_HEADER_LEN as usize];
            self.storage.read_exact_at(offset, &mut header)?;
            offset += RECORD_HEADER_LEN;
            let kind = header[0];
            let payload_len =
                u32::from_le_bytes([header[1], header[2], header[3], header[4]]) as usize;
            let expected_checksum =
                u32::from_le_bytes([header[5], header[6], header[7], header[8]]);

            if payload_len > MAX_RECORD_PAYLOAD_LEN {
                return Err(DbError::Decode(format!(
                    "derived db record at offset {} exceeds max payload length",
                    offset - RECORD_HEADER_LEN
                )));
            }
            if (payload_len as u64) > len.saturating_sub(offset) {
                if repair_tail {
                    self.storage.truncate(record_start)?;
                    self.storage.sync()?;
                    break;
                }
                return Err(DbError::Decode(format!(
                    "truncated derived db record payload at offset {}",
                    offset - RECORD_HEADER_LEN
                )));
            }

            let mut payload = alloc::vec![0u8; payload_len];
            self.storage.read_exact_at(offset, &mut payload)?;
            offset += payload_len as u64;
            let actual_checksum = record_checksum(kind, &payload);
            if actual_checksum != expected_checksum {
                return Err(DbError::Decode(format!(
                    "derived db record checksum mismatch at offset {}",
                    offset
                        .saturating_sub(payload_len as u64)
                        .saturating_sub(RECORD_HEADER_LEN)
                )));
            }
            self.apply_record(kind, &payload)?;
        }
        Ok(())
    }

    fn apply_record(&mut self, kind: u8, payload: &[u8]) -> Result<(), DbError> {
        match kind {
            RECORD_META => {
                let record = decode_meta_payload(payload)?;
                self.apply_meta(record.key, record.value, record.updated_at);
            }
            RECORD_DELETE_META => {
                let record = decode_key_payload(payload, "delete meta")?;
                self.runtime_meta.remove(&record.key);
            }
            RECORD_OBJECT_INDEX => {
                let record = decode_object_index_payload(payload)?;
                self.apply_object_index_owned(ObjectIndexRowOwned {
                    object_id: record.object_id,
                    stream_id: record.stream_id,
                    seq: record.seq,
                    representation_id: record.representation_id,
                    content_type: record.content_type,
                    updated_at: record.updated_at,
                });
            }
            RECORD_DELETE_OBJECT_INDEX => {
                let record = decode_key_payload(payload, "delete object index")?;
                self.remove_object_index(&record.key);
            }
            RECORD_ADMIN_AUDIT => {
                let record = decode_admin_audit_payload(payload)?;
                self.next_admin_audit_id =
                    self.next_admin_audit_id.max(record.id.saturating_add(1));
                self.push_admin_audit(AdminAuditRowOwned {
                    id: record.id,
                    event_time: record.event_time,
                    subject: record.subject,
                    action: record.action,
                    outcome: record.outcome,
                });
            }
            _ => return Err(DbError::Decode("unknown derived db record kind".into())),
        }
        Ok(())
    }

    fn apply_meta(&mut self, key: Vec<u8>, value: Vec<u8>, updated_at: i64) {
        self.runtime_meta.insert(
            key.clone(),
            RuntimeMetaRow {
                key,
                value,
                updated_at,
            },
        );
    }

    fn apply_object_index(&mut self, row: ObjectIndexRow<'_>) {
        self.apply_object_index_owned(ObjectIndexRowOwned {
            object_id: row.object_id.to_vec(),
            stream_id: row.stream_id.to_vec(),
            seq: row.seq,
            representation_id: row.representation_id.to_vec(),
            content_type: row.content_type.map(ToString::to_string),
            updated_at: row.updated_at,
        });
    }

    fn apply_object_index_owned(&mut self, row: ObjectIndexRowOwned) {
        self.remove_object_index(&row.object_id);
        self.object_index_by_stream
            .entry(row.stream_id.clone())
            .or_default()
            .push(row.object_id.clone());
        self.object_index_by_stream_seq
            .entry((row.stream_id.clone(), row.seq))
            .or_default()
            .push(row.object_id.clone());
        self.object_index.insert(row.object_id.clone(), row);
    }

    fn remove_object_index(&mut self, object_id: &[u8]) -> Option<ObjectIndexRowOwned> {
        let row = self.object_index.remove(object_id)?;
        remove_index_value(&mut self.object_index_by_stream, &row.stream_id, object_id);
        remove_index_value(
            &mut self.object_index_by_stream_seq,
            &(row.stream_id.clone(), row.seq),
            object_id,
        );
        Some(row)
    }

    fn apply_admin_audit(&mut self, id: u64, row: AdminAuditRow<'_>) {
        self.next_admin_audit_id = self.next_admin_audit_id.max(id.saturating_add(1));
        self.push_admin_audit(AdminAuditRowOwned {
            id,
            event_time: row.event_time,
            subject: row.subject.to_string(),
            action: row.action.to_string(),
            outcome: row.outcome.to_string(),
        });
    }

    fn push_admin_audit(&mut self, row: AdminAuditRowOwned) {
        self.admin_audit_by_id
            .insert(row.id, self.admin_audit.len());
        self.admin_audit.push(row);
    }

    fn apply_pending_record(&mut self, record: PendingRecord) {
        match record {
            PendingRecord::PutMeta(row) => self.apply_meta(row.key, row.value, row.updated_at),
            PendingRecord::DeleteMeta(key) => {
                self.runtime_meta.remove(&key);
            }
            PendingRecord::UpsertObjectIndex(row) => {
                self.apply_object_index_owned(row);
            }
            PendingRecord::DeleteObjectIndex(object_id) => {
                self.remove_object_index(&object_id);
            }
            PendingRecord::AppendAdminAudit(row) => {
                let id = self.next_admin_audit_id;
                self.next_admin_audit_id = self.next_admin_audit_id.saturating_add(1);
                self.push_admin_audit(AdminAuditRowOwned {
                    id,
                    event_time: row.event_time,
                    subject: row.subject,
                    action: row.action,
                    outcome: row.outcome,
                });
            }
        }
    }
}

impl ObjectIndexRow<'_> {
    fn to_owned(self) -> ObjectIndexRowOwned {
        ObjectIndexRowOwned {
            object_id: self.object_id.to_vec(),
            stream_id: self.stream_id.to_vec(),
            seq: self.seq,
            representation_id: self.representation_id.to_vec(),
            content_type: self.content_type.map(ToString::to_string),
            updated_at: self.updated_at,
        }
    }
}

impl AdminAuditRow<'_> {
    fn to_owned_draft(self) -> AdminAuditDraftOwned {
        AdminAuditDraftOwned {
            event_time: self.event_time,
            subject: self.subject.to_string(),
            action: self.action.to_string(),
            outcome: self.outcome.to_string(),
        }
    }
}

fn remove_index_value<K: Ord>(index: &mut BTreeMap<K, Vec<Vec<u8>>>, key: &K, value: &[u8]) {
    let should_remove = if let Some(values) = index.get_mut(key) {
        values.retain(|candidate| candidate.as_slice() != value);
        values.is_empty()
    } else {
        false
    };
    if should_remove {
        index.remove(key);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SqlTable {
    RuntimeMeta,
    DerivedObjectIndex,
    AdminAudit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SqlStatement {
    Select {
        columns: Vec<String>,
        table: SqlTable,
        filters: Vec<SqlFilter>,
        order_by: Option<SqlOrder>,
        limit: Option<usize>,
    },
    Insert {
        table: SqlTable,
        columns: Vec<String>,
    },
    Update {
        table: SqlTable,
        assignments: Vec<SqlAssignment>,
        filters: Vec<SqlFilter>,
    },
    Delete {
        table: SqlTable,
        filter: SqlFilter,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SqlFilter {
    column: String,
    op: SqlFilterOp,
    value: SqlFilterValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SqlFilterOp {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SqlAssignment {
    column: String,
    value: SqlFilterValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SqlOrder {
    column: String,
    descending: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SqlFilterValue {
    Param(usize),
    Literal(SqlValue),
}

fn explain_statement(statement: &SqlStatement) -> SqlExplain {
    match statement {
        SqlStatement::Select {
            table,
            filters,
            order_by,
            limit,
            ..
        } => SqlExplain {
            table: table_schema(*table).name,
            access_path: choose_access_path(*table, filters),
            ordered: order_by.is_some(),
            limit: *limit,
        },
        SqlStatement::Insert { table, .. } => SqlExplain {
            table: table_schema(*table).name,
            access_path: SqlAccessPath::FullScan,
            ordered: false,
            limit: None,
        },
        SqlStatement::Update { table, filters, .. } => SqlExplain {
            table: table_schema(*table).name,
            access_path: choose_access_path(*table, filters),
            ordered: false,
            limit: None,
        },
        SqlStatement::Delete { table, filter } => {
            let filters = alloc::vec![filter.clone()];
            SqlExplain {
                table: table_schema(*table).name,
                access_path: choose_access_path(*table, &filters),
                ordered: false,
                limit: None,
            }
        }
    }
}

fn choose_access_path(table: SqlTable, filters: &[SqlFilter]) -> SqlAccessPath {
    match table {
        SqlTable::RuntimeMeta if has_eq_filter(filters, "key") => SqlAccessPath::RuntimeMetaKey,
        SqlTable::DerivedObjectIndex if has_eq_filter(filters, "object_id") => {
            SqlAccessPath::ObjectIndexObjectId
        }
        SqlTable::DerivedObjectIndex
            if has_eq_filter(filters, "stream_id") && has_eq_filter(filters, "seq") =>
        {
            SqlAccessPath::ObjectIndexStreamSeq
        }
        SqlTable::DerivedObjectIndex if has_eq_filter(filters, "stream_id") => {
            SqlAccessPath::ObjectIndexStream
        }
        SqlTable::AdminAudit if has_eq_filter(filters, "id") => SqlAccessPath::AdminAuditId,
        _ => SqlAccessPath::FullScan,
    }
}

fn has_eq_filter(filters: &[SqlFilter], column: &str) -> bool {
    filters
        .iter()
        .any(|filter| filter.column == column && filter.op == SqlFilterOp::Eq)
}

fn stage_sql_write(
    statement: &SqlStatement,
    params: &[SqlValue],
    runtime_meta: &mut BTreeMap<Vec<u8>, RuntimeMetaRow>,
    object_index: &mut BTreeMap<Vec<u8>, ObjectIndexRowOwned>,
) -> Result<(Option<PendingRecord>, usize), DbError> {
    match statement {
        SqlStatement::Select { .. } => Err(DbError::Sql(
            "SQL batch only supports INSERT, UPDATE, and DELETE".into(),
        )),
        SqlStatement::Insert { table, columns } => {
            let record = stage_insert(*table, columns, params, runtime_meta, object_index)?;
            Ok((Some(record), 1))
        }
        SqlStatement::Update {
            table,
            assignments,
            filters,
        } => stage_update(
            *table,
            assignments,
            filters,
            params,
            runtime_meta,
            object_index,
        ),
        SqlStatement::Delete { table, filter } => {
            let (record, affected) =
                stage_delete(*table, filter, params, runtime_meta, object_index)?;
            Ok((Some(record), affected))
        }
    }
}

fn stage_insert(
    table: SqlTable,
    columns: &[String],
    params: &[SqlValue],
    runtime_meta: &mut BTreeMap<Vec<u8>, RuntimeMetaRow>,
    object_index: &mut BTreeMap<Vec<u8>, ObjectIndexRowOwned>,
) -> Result<PendingRecord, DbError> {
    if columns.len() != params.len() {
        return Err(DbError::Sql("insert column/value count mismatch".into()));
    }
    validate_input_columns(table, columns)?;
    match table {
        SqlTable::RuntimeMeta => {
            let row = runtime_meta_row_from_columns(columns, params)?;
            runtime_meta.insert(row.key.clone(), row.clone());
            Ok(PendingRecord::PutMeta(row))
        }
        SqlTable::DerivedObjectIndex => {
            let row = object_index_row_from_columns(columns, params)?;
            object_index.insert(row.object_id.clone(), row.clone());
            Ok(PendingRecord::UpsertObjectIndex(row))
        }
        SqlTable::AdminAudit => {
            let row = admin_audit_draft_from_columns(columns, params)?;
            Ok(PendingRecord::AppendAdminAudit(row))
        }
    }
}

fn runtime_meta_row_from_columns(
    columns: &[String],
    params: &[SqlValue],
) -> Result<RuntimeMetaRow, DbError> {
    Ok(RuntimeMetaRow {
        key: required_column_bytes(columns, params, "key")?.to_vec(),
        value: required_column_bytes(columns, params, "value")?.to_vec(),
        updated_at: required_column_i64(columns, params, "updated_at")?,
    })
}

fn object_index_row_from_columns(
    columns: &[String],
    params: &[SqlValue],
) -> Result<ObjectIndexRowOwned, DbError> {
    let row = ObjectIndexRowOwned {
        object_id: required_column_bytes(columns, params, "object_id")?.to_vec(),
        stream_id: required_column_bytes(columns, params, "stream_id")?.to_vec(),
        seq: required_column_u64(columns, params, "seq")?,
        representation_id: required_column_bytes(columns, params, "representation_id")?.to_vec(),
        content_type: optional_column_str(columns, params, "content_type")?
            .map(ToString::to_string),
        updated_at: required_column_i64(columns, params, "updated_at")?,
    };
    validate_object_index_row(row.as_borrowed())?;
    Ok(row)
}

fn admin_audit_draft_from_columns(
    columns: &[String],
    params: &[SqlValue],
) -> Result<AdminAuditDraftOwned, DbError> {
    Ok(AdminAuditDraftOwned {
        event_time: required_column_i64(columns, params, "event_time")?,
        subject: required_column_str(columns, params, "subject")?.to_string(),
        action: required_column_str(columns, params, "action")?.to_string(),
        outcome: required_column_str(columns, params, "outcome")?.to_string(),
    })
}

fn stage_update(
    table: SqlTable,
    assignments: &[SqlAssignment],
    filters: &[SqlFilter],
    params: &[SqlValue],
    runtime_meta: &mut BTreeMap<Vec<u8>, RuntimeMetaRow>,
    object_index: &mut BTreeMap<Vec<u8>, ObjectIndexRowOwned>,
) -> Result<(Option<PendingRecord>, usize), DbError> {
    if assignments.is_empty() {
        return Err(DbError::Sql(
            "UPDATE requires at least one assignment".into(),
        ));
    }
    if table_schema(table).append_only {
        return Err(DbError::Sql(
            "admin_audit is append-only and cannot be updated".into(),
        ));
    }
    validate_assignment_columns(table, assignments)?;

    match table {
        SqlTable::RuntimeMeta => {
            let filter = require_single_filter(filters)?;
            require_filter_column(filter, "key")?;
            let key = filter_value(filter, params)?.as_bytes().ok_or_else(|| {
                DbError::Sql("runtime_meta.key filter requires bytes or text".into())
            })?;
            let Some(mut row) = runtime_meta.get(key).cloned() else {
                return Ok((None, 0));
            };
            for assignment in assignments {
                apply_runtime_meta_assignment(&mut row, assignment, params)?;
            }
            runtime_meta.insert(row.key.clone(), row.clone());
            Ok((Some(PendingRecord::PutMeta(row)), 1))
        }
        SqlTable::DerivedObjectIndex => {
            let filter = require_single_filter(filters)?;
            require_filter_column(filter, "object_id")?;
            let object_id = filter_value(filter, params)?.as_bytes().ok_or_else(|| {
                DbError::Sql("derived_object_index.object_id filter requires bytes or text".into())
            })?;
            let Some(mut row) = object_index.get(object_id).cloned() else {
                return Ok((None, 0));
            };
            for assignment in assignments {
                apply_object_index_assignment(&mut row, assignment, params)?;
            }
            validate_object_index_row(row.as_borrowed())?;
            object_index.insert(row.object_id.clone(), row.clone());
            Ok((Some(PendingRecord::UpsertObjectIndex(row)), 1))
        }
        SqlTable::AdminAudit => unreachable!(),
    }
}

fn apply_runtime_meta_assignment(
    row: &mut RuntimeMetaRow,
    assignment: &SqlAssignment,
    params: &[SqlValue],
) -> Result<(), DbError> {
    let value = assignment_value(assignment, params)?;
    match assignment.column.as_str() {
        "value" => {
            row.value = value
                .as_bytes()
                .ok_or_else(|| {
                    DbError::Sql("runtime_meta.value assignment requires bytes or text".into())
                })?
                .to_vec();
        }
        "updated_at" => {
            row.updated_at = value.as_i64().ok_or_else(|| {
                DbError::Sql("runtime_meta.updated_at assignment requires integer".into())
            })?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn apply_object_index_assignment(
    row: &mut ObjectIndexRowOwned,
    assignment: &SqlAssignment,
    params: &[SqlValue],
) -> Result<(), DbError> {
    let value = assignment_value(assignment, params)?;
    match assignment.column.as_str() {
        "stream_id" => {
            row.stream_id = value
                .as_bytes()
                .ok_or_else(|| {
                    DbError::Sql(
                        "derived_object_index.stream_id assignment requires bytes or text".into(),
                    )
                })?
                .to_vec();
        }
        "seq" => {
            row.seq = non_negative_assignment_u64(value, "derived_object_index.seq")?;
        }
        "representation_id" => {
            row.representation_id = value
                .as_bytes()
                .ok_or_else(|| {
                    DbError::Sql(
                        "derived_object_index.representation_id assignment requires bytes or text"
                            .into(),
                    )
                })?
                .to_vec();
        }
        "content_type" => {
            row.content_type =
                optional_text_assignment(value, "derived_object_index.content_type")?;
        }
        "updated_at" => {
            row.updated_at = value.as_i64().ok_or_else(|| {
                DbError::Sql("derived_object_index.updated_at assignment requires integer".into())
            })?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn stage_delete(
    table: SqlTable,
    filter: &SqlFilter,
    params: &[SqlValue],
    runtime_meta: &mut BTreeMap<Vec<u8>, RuntimeMetaRow>,
    object_index: &mut BTreeMap<Vec<u8>, ObjectIndexRowOwned>,
) -> Result<(PendingRecord, usize), DbError> {
    match table {
        SqlTable::RuntimeMeta => {
            require_filter_column(filter, "key")?;
            let key = filter_value(filter, params)?.as_bytes().ok_or_else(|| {
                DbError::Sql("runtime_meta.key filter requires bytes or text".into())
            })?;
            let affected = usize::from(runtime_meta.remove(key).is_some());
            Ok((PendingRecord::DeleteMeta(key.to_vec()), affected))
        }
        SqlTable::DerivedObjectIndex => {
            require_filter_column(filter, "object_id")?;
            let object_id = filter_value(filter, params)?.as_bytes().ok_or_else(|| {
                DbError::Sql("derived_object_index.object_id filter requires bytes or text".into())
            })?;
            let affected = usize::from(object_index.remove(object_id).is_some());
            Ok((
                PendingRecord::DeleteObjectIndex(object_id.to_vec()),
                affected,
            ))
        }
        SqlTable::AdminAudit => Err(DbError::Sql(
            "admin_audit is append-only and cannot be deleted".into(),
        )),
    }
}

fn parse_sql(sql: &str) -> Result<SqlStatement, DbError> {
    let sql = sql.trim().trim_end_matches(';').trim();
    if sql.is_empty() {
        return Err(DbError::Sql("empty statement".into()));
    }
    let upper = sql.to_ascii_uppercase();
    if upper.starts_with("SELECT ") {
        parse_select(sql)
    } else if upper.starts_with("INSERT INTO ") {
        parse_insert(sql)
    } else if upper.starts_with("UPDATE ") {
        parse_update(sql)
    } else if upper.starts_with("DELETE FROM ") {
        parse_delete(sql)
    } else {
        Err(DbError::Sql(
            "supported SQL starts with SELECT, INSERT INTO, UPDATE, or DELETE FROM".into(),
        ))
    }
}

fn parse_select(sql: &str) -> Result<SqlStatement, DbError> {
    let rest = sql[6..].trim();
    let (columns_part, after_from) = split_keyword(rest, "FROM")?;
    let (after_from, limit_part) =
        split_optional_keyword(after_from.trim(), "LIMIT")?.unwrap_or((after_from.trim(), ""));
    let (after_from, order_part) =
        split_optional_keyword(after_from.trim(), "ORDER BY")?.unwrap_or((after_from.trim(), ""));
    let (table_part, where_part) =
        split_optional_keyword(after_from.trim(), "WHERE")?.unwrap_or((after_from.trim(), ""));
    let columns = parse_column_list(columns_part)?;
    let table = parse_table(table_part.trim())?;
    let filters = if where_part.trim().is_empty() {
        Vec::new()
    } else {
        parse_filters(where_part)?
    };
    let order_by = parse_order_by(order_part)?;
    let limit = parse_limit(limit_part)?;
    Ok(SqlStatement::Select {
        columns,
        table,
        filters,
        order_by,
        limit,
    })
}

fn parse_insert(sql: &str) -> Result<SqlStatement, DbError> {
    let rest = sql["INSERT INTO ".len()..].trim();
    let open = rest
        .find('(')
        .ok_or_else(|| DbError::Sql("INSERT requires column list".into()))?;
    let table = parse_table(rest[..open].trim())?;
    let close = rest[open + 1..]
        .find(')')
        .map(|idx| open + 1 + idx)
        .ok_or_else(|| DbError::Sql("INSERT column list is missing ')'".into()))?;
    let columns = parse_column_list(&rest[open + 1..close])?;
    let after_columns = rest[close + 1..].trim();
    let upper_after = after_columns.to_ascii_uppercase();
    if !upper_after.starts_with("VALUES") {
        return Err(DbError::Sql("INSERT requires VALUES".into()));
    }
    let values = after_columns["VALUES".len()..].trim();
    if !values.starts_with('(') || !values.ends_with(')') {
        return Err(DbError::Sql(
            "INSERT VALUES must be placeholders in (...)".into(),
        ));
    }
    let value_count = values[1..values.len() - 1]
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .try_fold(0usize, |count, value| {
            if value == "?" {
                Ok(count + 1)
            } else {
                Err(DbError::Sql(
                    "INSERT VALUES only supports '?' parameters".into(),
                ))
            }
        })?;
    if value_count != columns.len() {
        return Err(DbError::Sql("INSERT column/value count mismatch".into()));
    }
    Ok(SqlStatement::Insert { table, columns })
}

fn parse_update(sql: &str) -> Result<SqlStatement, DbError> {
    let rest = sql["UPDATE ".len()..].trim();
    let (table_part, after_set) = split_keyword(rest, "SET")?;
    let (set_part, where_part) = split_optional_keyword(after_set, "WHERE")?
        .ok_or_else(|| DbError::Sql("UPDATE requires WHERE".into()))?;
    let (assignments, next_param) = parse_assignments(set_part, 0)?;
    let filters = parse_filters_with_start(where_part, next_param)?;
    Ok(SqlStatement::Update {
        table: parse_table(table_part.trim())?,
        assignments,
        filters,
    })
}

fn parse_delete(sql: &str) -> Result<SqlStatement, DbError> {
    let rest = sql["DELETE FROM ".len()..].trim();
    let (table_part, where_part) = split_optional_keyword(rest, "WHERE")?
        .ok_or_else(|| DbError::Sql("DELETE requires WHERE".into()))?;
    let (filter, _) = parse_filter(where_part, 0)?;
    Ok(SqlStatement::Delete {
        table: parse_table(table_part.trim())?,
        filter,
    })
}

fn split_keyword<'a>(input: &'a str, keyword: &str) -> Result<(&'a str, &'a str), DbError> {
    split_optional_keyword(input, keyword)?
        .ok_or_else(|| DbError::Sql(format!("missing {keyword}")))
}

fn split_optional_keyword<'a>(
    input: &'a str,
    keyword: &str,
) -> Result<Option<(&'a str, &'a str)>, DbError> {
    if let Some((idx, len)) = find_keyword_outside_quotes(input, keyword)? {
        let before = &input[..idx];
        let after = &input[idx + len..];
        Ok(Some((before, after)))
    } else {
        Ok(None)
    }
}

fn find_keyword_outside_quotes(
    input: &str,
    keyword: &str,
) -> Result<Option<(usize, usize)>, DbError> {
    let keyword = keyword.as_bytes();
    scan_outside_quotes(input, |idx| {
        keyword_matches_at(input, idx, keyword).then_some(keyword.len())
    })
}

fn keyword_matches_at(input: &str, idx: usize, keyword: &[u8]) -> bool {
    let bytes = input.as_bytes();
    if idx + keyword.len() > bytes.len() {
        return false;
    }
    if idx > 0 && !bytes[idx - 1].is_ascii_whitespace() {
        return false;
    }
    if idx + keyword.len() < bytes.len() && !bytes[idx + keyword.len()].is_ascii_whitespace() {
        return false;
    }
    bytes[idx..idx + keyword.len()]
        .iter()
        .zip(keyword)
        .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

fn split_delimited_outside_quotes(input: &str, delimiter: char) -> Result<Vec<&str>, DbError> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let delimiter = delimiter as u8;
    let bytes = input.as_bytes();
    while let Some((idx, _len)) = scan_outside_quotes(&input[start..], |idx| {
        (bytes[start + idx] == delimiter).then_some(1)
    })? {
        let idx = start + idx;
        parts.push(input[start..idx].trim());
        start = idx + 1;
    }
    parts.push(input[start..].trim());
    Ok(parts)
}

fn split_once_char_outside_quotes(
    input: &str,
    delimiter: char,
) -> Result<Option<(&str, &str)>, DbError> {
    let delimiter = delimiter as u8;
    let bytes = input.as_bytes();
    Ok(
        scan_outside_quotes(input, |idx| (bytes[idx] == delimiter).then_some(1))?
            .map(|(idx, _len)| (&input[..idx], &input[idx + 1..])),
    )
}

fn split_once_token_outside_quotes<'a>(
    input: &'a str,
    token: &str,
) -> Result<Option<(&'a str, &'a str)>, DbError> {
    let token = token.as_bytes();
    let bytes = input.as_bytes();
    Ok(scan_outside_quotes(input, |idx| {
        (idx + token.len() <= bytes.len() && &bytes[idx..idx + token.len()] == token)
            .then_some(token.len())
    })?
    .map(|(idx, len)| (&input[..idx], &input[idx + len..])))
}

fn scan_outside_quotes(
    input: &str,
    mut matches_at: impl FnMut(usize) -> Option<usize>,
) -> Result<Option<(usize, usize)>, DbError> {
    let bytes = input.as_bytes();
    let mut in_quote = false;
    let mut idx = 0usize;
    while idx < bytes.len() {
        match bytes[idx] {
            b'\'' => {
                if in_quote && idx + 1 < bytes.len() && bytes[idx + 1] == b'\'' {
                    idx += 2;
                } else {
                    in_quote = !in_quote;
                    idx += 1;
                }
            }
            _ if !in_quote => {
                if let Some(len) = matches_at(idx) {
                    return Ok(Some((idx, len)));
                }
                idx += 1;
            }
            _ => idx += 1,
        }
    }
    if in_quote {
        return Err(DbError::Sql("unterminated string literal".into()));
    }
    Ok(None)
}

fn parse_column_list(input: &str) -> Result<Vec<String>, DbError> {
    let mut columns = Vec::new();
    for column in split_delimited_outside_quotes(input, ',')? {
        let trimmed = column.trim();
        let column = if trimmed.eq_ignore_ascii_case("COUNT(*)") {
            "count(*)".to_string()
        } else {
            normalize_ident(trimmed)?
        };
        columns.push(column);
    }
    if columns.is_empty() {
        return Err(DbError::Sql("empty column list".into()));
    }
    Ok(columns)
}

fn parse_filters(input: &str) -> Result<Vec<SqlFilter>, DbError> {
    parse_filters_with_start(input, 0)
}

fn parse_filters_with_start(input: &str, start_param: usize) -> Result<Vec<SqlFilter>, DbError> {
    let mut filters = Vec::new();
    let mut next_param = start_param;
    for part in split_and_terms(input)? {
        let (filter, consumed_unnumbered) = parse_filter(part, next_param)?;
        if consumed_unnumbered {
            next_param += 1;
        }
        filters.push(filter);
    }
    if filters.is_empty() {
        return Err(DbError::Sql("WHERE has no predicates".into()));
    }
    Ok(filters)
}

fn parse_assignments(
    input: &str,
    start_param: usize,
) -> Result<(Vec<SqlAssignment>, usize), DbError> {
    let mut assignments = Vec::new();
    let mut next_param = start_param;
    for part in split_delimited_outside_quotes(input, ',')? {
        let (column, value) = split_once_char_outside_quotes(part, '=')?
            .ok_or_else(|| DbError::Sql("SET only supports column = value".into()))?;
        let column = normalize_ident(column.trim())?;
        let (value, consumed_unnumbered) = parse_filter_value(value.trim(), next_param)?;
        if consumed_unnumbered {
            next_param += 1;
        }
        assignments.push(SqlAssignment { column, value });
    }
    if assignments.is_empty() {
        return Err(DbError::Sql(
            "UPDATE requires at least one assignment".into(),
        ));
    }
    Ok((assignments, next_param))
}

fn split_and_terms(input: &str) -> Result<Vec<&str>, DbError> {
    let mut out = Vec::new();
    let mut start = 0usize;
    while let Some((relative, len)) = find_keyword_outside_quotes(&input[start..], "AND")? {
        let idx = start + relative;
        out.push(input[start..idx].trim());
        start = idx + len;
    }
    out.push(input[start..].trim());
    Ok(out)
}

fn parse_filter(input: &str, next_param: usize) -> Result<(SqlFilter, bool), DbError> {
    let (column, op, value) = split_filter_comparison(input)?.ok_or_else(|| {
        DbError::Sql("WHERE only supports column comparisons with =, >, >=, <, or <=".into())
    })?;
    let column = normalize_ident(column.trim())?;
    let (value, consumed_unnumbered) = parse_filter_value(value.trim(), next_param)?;
    Ok((SqlFilter { column, op, value }, consumed_unnumbered))
}

fn split_filter_comparison(input: &str) -> Result<Option<(&str, SqlFilterOp, &str)>, DbError> {
    for (token, op) in [
        (">=", SqlFilterOp::Gte),
        ("<=", SqlFilterOp::Lte),
        ("=", SqlFilterOp::Eq),
        (">", SqlFilterOp::Gt),
        ("<", SqlFilterOp::Lt),
    ] {
        if let Some((left, right)) = split_once_token_outside_quotes(input, token)? {
            return Ok(Some((left, op, right)));
        }
    }
    Ok(None)
}

fn parse_limit(input: &str) -> Result<Option<usize>, DbError> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(None);
    }
    if input.contains(char::is_whitespace) {
        return Err(DbError::Sql("LIMIT only supports one integer".into()));
    }
    let value = input
        .parse::<usize>()
        .map_err(|_| DbError::Sql("LIMIT requires a non-negative integer".into()))?;
    Ok(Some(value))
}

fn parse_order_by(input: &str) -> Result<Option<SqlOrder>, DbError> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(None);
    }
    let mut parts = input.split_whitespace();
    let column = parts
        .next()
        .ok_or_else(|| DbError::Sql("ORDER BY requires a column".into()))
        .and_then(normalize_ident)?;
    let descending = match parts.next() {
        None => false,
        Some(direction) if direction.eq_ignore_ascii_case("ASC") => false,
        Some(direction) if direction.eq_ignore_ascii_case("DESC") => true,
        Some(_) => {
            return Err(DbError::Sql(
                "ORDER BY only supports one column with optional ASC or DESC".into(),
            ));
        }
    };
    if parts.next().is_some() {
        return Err(DbError::Sql(
            "ORDER BY only supports one column with optional ASC or DESC".into(),
        ));
    }
    Ok(Some(SqlOrder { column, descending }))
}

fn parse_filter_value(input: &str, next_param: usize) -> Result<(SqlFilterValue, bool), DbError> {
    if input == "?" {
        return Ok((SqlFilterValue::Param(next_param), true));
    }
    if let Some(numbered) = input.strip_prefix('?') {
        let idx = numbered
            .parse::<usize>()
            .map_err(|_| DbError::Sql("invalid numbered parameter".into()))?;
        if idx == 0 {
            return Err(DbError::Sql("numbered parameters are 1-based".into()));
        }
        return Ok((SqlFilterValue::Param(idx - 1), false));
    }
    if input.eq_ignore_ascii_case("NULL") {
        return Ok((SqlFilterValue::Literal(SqlValue::Null), false));
    }
    if let Some(text) = parse_quoted(input)? {
        return Ok((SqlFilterValue::Literal(SqlValue::Text(text)), false));
    }
    let value = input
        .parse::<i64>()
        .map_err(|_| DbError::Sql("unsupported WHERE literal".into()))?;
    Ok((SqlFilterValue::Literal(SqlValue::Integer(value)), false))
}

fn parse_quoted(input: &str) -> Result<Option<String>, DbError> {
    if !input.starts_with('\'') {
        return Ok(None);
    }
    if !input.ends_with('\'') || input.len() < 2 {
        return Err(DbError::Sql("unterminated string literal".into()));
    }
    let mut out = String::new();
    let mut chars = input[1..input.len() - 1].chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\'' && chars.peek() == Some(&'\'') {
            chars.next();
            out.push('\'');
        } else {
            out.push(ch);
        }
    }
    Ok(Some(out))
}

fn parse_table(input: &str) -> Result<SqlTable, DbError> {
    match normalize_ident(input)?.as_str() {
        "runtime_meta" => Ok(SqlTable::RuntimeMeta),
        "derived_object_index" => Ok(SqlTable::DerivedObjectIndex),
        "admin_audit" => Ok(SqlTable::AdminAudit),
        table => Err(DbError::Sql(format!("unknown table {table}"))),
    }
}

fn normalize_ident(input: &str) -> Result<String, DbError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DbError::Sql("empty identifier".into()));
    }
    if input == "*" {
        return Ok(input.to_string());
    }
    if !input
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(DbError::Sql(format!("invalid identifier {input}")));
    }
    Ok(input.to_ascii_lowercase())
}

fn expand_select_columns(table: SqlTable, columns: Vec<String>) -> Result<Vec<String>, DbError> {
    if columns.len() == 1 && columns[0] == "*" {
        return Ok(default_columns(table)
            .iter()
            .map(|column| (*column).to_string())
            .collect());
    }
    for column in &columns {
        validate_column(table, column)?;
    }
    Ok(columns)
}

fn is_count_star(columns: &[String]) -> bool {
    columns.len() == 1 && columns[0] == "count(*)"
}

fn default_columns(table: SqlTable) -> &'static [&'static str] {
    match table {
        SqlTable::RuntimeMeta => &["key", "value", "updated_at"],
        SqlTable::DerivedObjectIndex => &[
            "object_id",
            "stream_id",
            "seq",
            "representation_id",
            "content_type",
            "updated_at",
        ],
        SqlTable::AdminAudit => &["id", "event_time", "subject", "action", "outcome"],
    }
}

fn table_schema(table: SqlTable) -> &'static SqlTableSchema {
    match table {
        SqlTable::RuntimeMeta => &SQL_SCHEMA[0],
        SqlTable::DerivedObjectIndex => &SQL_SCHEMA[1],
        SqlTable::AdminAudit => &SQL_SCHEMA[2],
    }
}

fn validate_column(table: SqlTable, column: &str) -> Result<(), DbError> {
    if table_schema(table)
        .columns
        .iter()
        .any(|schema| schema.name == column)
    {
        Ok(())
    } else {
        Err(DbError::Sql(format!("unknown column {column}")))
    }
}

fn validate_input_columns(table: SqlTable, columns: &[String]) -> Result<(), DbError> {
    for column in columns {
        validate_column(table, column)?;
    }
    reject_duplicate_columns(columns)
}

fn validate_assignment_columns(
    table: SqlTable,
    assignments: &[SqlAssignment],
) -> Result<(), DbError> {
    let mut columns = Vec::new();
    for assignment in assignments {
        let Some(schema) = table_schema(table)
            .columns
            .iter()
            .find(|schema| schema.name == assignment.column)
        else {
            return Err(DbError::Sql(format!(
                "unknown column {}",
                assignment.column
            )));
        };
        if !schema.mutable {
            return Err(DbError::Sql(format!(
                "column {} cannot be updated",
                assignment.column
            )));
        }
        columns.push(assignment.column.clone());
    }
    reject_duplicate_columns(&columns)
}

fn reject_duplicate_columns(columns: &[String]) -> Result<(), DbError> {
    for (idx, column) in columns.iter().enumerate() {
        if columns[idx + 1..]
            .iter()
            .any(|candidate| candidate == column)
        {
            return Err(DbError::Sql(format!("duplicate column {column}")));
        }
    }
    Ok(())
}

fn project_runtime_meta_row(
    row: &RuntimeMetaRow,
    columns: &[String],
) -> Result<Vec<SqlValue>, DbError> {
    columns
        .iter()
        .map(|column| runtime_meta_column_value(row, column))
        .collect()
}

fn runtime_meta_column_value(row: &RuntimeMetaRow, column: &str) -> Result<SqlValue, DbError> {
    match column {
        "key" => Ok(SqlValue::Bytes(row.key.clone())),
        "value" => Ok(SqlValue::Bytes(row.value.clone())),
        "updated_at" => Ok(SqlValue::Integer(row.updated_at)),
        _ => Err(DbError::Sql(format!(
            "unknown runtime_meta column {column}"
        ))),
    }
}

fn project_object_index_row(
    row: &ObjectIndexRowOwned,
    columns: &[String],
) -> Result<Vec<SqlValue>, DbError> {
    columns
        .iter()
        .map(|column| object_index_column_value(row, column))
        .collect()
}

fn object_index_column_value(row: &ObjectIndexRowOwned, column: &str) -> Result<SqlValue, DbError> {
    match column {
        "object_id" => Ok(SqlValue::Bytes(row.object_id.clone())),
        "stream_id" => Ok(SqlValue::Bytes(row.stream_id.clone())),
        "seq" => Ok(SqlValue::Integer(row.seq as i64)),
        "representation_id" => Ok(SqlValue::Bytes(row.representation_id.clone())),
        "content_type" => Ok(row
            .content_type
            .as_ref()
            .map(|value| SqlValue::Text(value.clone()))
            .unwrap_or(SqlValue::Null)),
        "updated_at" => Ok(SqlValue::Integer(row.updated_at)),
        _ => Err(DbError::Sql(format!(
            "unknown derived_object_index column {column}"
        ))),
    }
}

fn project_admin_audit_row(
    row: &AdminAuditRowOwned,
    columns: &[String],
) -> Result<Vec<SqlValue>, DbError> {
    columns
        .iter()
        .map(|column| admin_audit_column_value(row, column))
        .collect()
}

fn admin_audit_column_value(row: &AdminAuditRowOwned, column: &str) -> Result<SqlValue, DbError> {
    match column {
        "id" => Ok(SqlValue::Integer(row.id as i64)),
        "event_time" => Ok(SqlValue::Integer(row.event_time)),
        "subject" => Ok(SqlValue::Text(row.subject.clone())),
        "action" => Ok(SqlValue::Text(row.action.clone())),
        "outcome" => Ok(SqlValue::Text(row.outcome.clone())),
        _ => Err(DbError::Sql(format!("unknown admin_audit column {column}"))),
    }
}

fn compare_sql_values(left: &SqlValue, right: &SqlValue) -> Ordering {
    match (left, right) {
        (SqlValue::Null, SqlValue::Null) => Ordering::Equal,
        (SqlValue::Null, _) => Ordering::Less,
        (_, SqlValue::Null) => Ordering::Greater,
        (SqlValue::Integer(left), SqlValue::Integer(right)) => left.cmp(right),
        (SqlValue::Bytes(left), SqlValue::Bytes(right)) => left.cmp(right),
        (SqlValue::Text(left), SqlValue::Text(right)) => left.cmp(right),
        _ => sql_value_rank(left).cmp(&sql_value_rank(right)),
    }
}

fn sql_value_rank(value: &SqlValue) -> u8 {
    match value {
        SqlValue::Null => 0,
        SqlValue::Integer(_) => 1,
        SqlValue::Bytes(_) => 2,
        SqlValue::Text(_) => 3,
    }
}

fn matches_runtime_meta_filters(
    row: &RuntimeMetaRow,
    filters: &[SqlFilter],
    params: &[SqlValue],
) -> Result<bool, DbError> {
    for filter in filters {
        validate_column(SqlTable::RuntimeMeta, &filter.column)?;
        let value = filter_value(filter, params)?;
        let matches = match filter.column.as_str() {
            "key" => matches_bytes_filter(&filter.op, &row.key, value)?,
            "value" => matches_bytes_filter(&filter.op, &row.value, value)?,
            "updated_at" => matches_i64_filter(&filter.op, row.updated_at, value)?,
            _ => false,
        };
        if !matches {
            return Ok(false);
        }
    }
    Ok(true)
}

fn matches_object_index_filters(
    row: &ObjectIndexRowOwned,
    filters: &[SqlFilter],
    params: &[SqlValue],
) -> Result<bool, DbError> {
    for filter in filters {
        validate_column(SqlTable::DerivedObjectIndex, &filter.column)?;
        let value = filter_value(filter, params)?;
        let matches = match filter.column.as_str() {
            "object_id" => matches_bytes_filter(&filter.op, &row.object_id, value)?,
            "stream_id" => matches_bytes_filter(&filter.op, &row.stream_id, value)?,
            "seq" => matches_u64_filter(&filter.op, row.seq, value)?,
            "representation_id" => matches_bytes_filter(&filter.op, &row.representation_id, value)?,
            "content_type" => {
                matches_optional_text_filter(&filter.op, row.content_type.as_deref(), value)?
            }
            "updated_at" => matches_i64_filter(&filter.op, row.updated_at, value)?,
            _ => false,
        };
        if !matches {
            return Ok(false);
        }
    }
    Ok(true)
}

fn matches_admin_audit_filters(
    row: &AdminAuditRowOwned,
    filters: &[SqlFilter],
    params: &[SqlValue],
) -> Result<bool, DbError> {
    for filter in filters {
        validate_column(SqlTable::AdminAudit, &filter.column)?;
        let value = filter_value(filter, params)?;
        let matches = match filter.column.as_str() {
            "id" => matches_u64_filter(&filter.op, row.id, value)?,
            "event_time" => matches_i64_filter(&filter.op, row.event_time, value)?,
            "subject" => matches_text_filter(&filter.op, &row.subject, value)?,
            "action" => matches_text_filter(&filter.op, &row.action, value)?,
            "outcome" => matches_text_filter(&filter.op, &row.outcome, value)?,
            _ => false,
        };
        if !matches {
            return Ok(false);
        }
    }
    Ok(true)
}

fn matches_bytes_filter(op: &SqlFilterOp, left: &[u8], right: &SqlValue) -> Result<bool, DbError> {
    require_eq_filter(op, "bytes")?;
    Ok(right.as_bytes().is_some_and(|right| right == left))
}

fn matches_text_filter(op: &SqlFilterOp, left: &str, right: &SqlValue) -> Result<bool, DbError> {
    require_eq_filter(op, "text")?;
    Ok(right.as_str().is_some_and(|right| right == left))
}

fn matches_optional_text_filter(
    op: &SqlFilterOp,
    left: Option<&str>,
    right: &SqlValue,
) -> Result<bool, DbError> {
    require_eq_filter(op, "text")?;
    Ok(match (left, right) {
        (None, SqlValue::Null) => true,
        (Some(left), SqlValue::Text(right)) => left == right,
        _ => false,
    })
}

fn matches_i64_filter(op: &SqlFilterOp, left: i64, right: &SqlValue) -> Result<bool, DbError> {
    let Some(right) = right.as_i64() else {
        return Ok(false);
    };
    Ok(match op {
        SqlFilterOp::Eq => left == right,
        SqlFilterOp::Gt => left > right,
        SqlFilterOp::Gte => left >= right,
        SqlFilterOp::Lt => left < right,
        SqlFilterOp::Lte => left <= right,
    })
}

fn matches_u64_filter(op: &SqlFilterOp, left: u64, right: &SqlValue) -> Result<bool, DbError> {
    let Some(right) = right.as_i64() else {
        return Ok(false);
    };
    if right < 0 {
        return Ok(false);
    }
    let right = right as u64;
    Ok(match op {
        SqlFilterOp::Eq => left == right,
        SqlFilterOp::Gt => left > right,
        SqlFilterOp::Gte => left >= right,
        SqlFilterOp::Lt => left < right,
        SqlFilterOp::Lte => left <= right,
    })
}

fn require_eq_filter(op: &SqlFilterOp, ty: &str) -> Result<(), DbError> {
    if *op == SqlFilterOp::Eq {
        Ok(())
    } else {
        Err(DbError::Sql(format!(
            "{ty} columns only support equality predicates"
        )))
    }
}

fn filter_value<'a>(
    filter: &'a SqlFilter,
    params: &'a [SqlValue],
) -> Result<&'a SqlValue, DbError> {
    sql_value(&filter.value, params)
}

fn filter_value_for_column<'a>(
    filters: &'a [SqlFilter],
    params: &'a [SqlValue],
    column: &str,
) -> Result<Option<&'a SqlValue>, DbError> {
    filters
        .iter()
        .find(|filter| filter.column == column)
        .map(|filter| filter_value(filter, params))
        .transpose()
}

fn eq_filter_value_for_column<'a>(
    filters: &'a [SqlFilter],
    params: &'a [SqlValue],
    column: &str,
) -> Result<Option<&'a SqlValue>, DbError> {
    filters
        .iter()
        .find(|filter| filter.column == column && filter.op == SqlFilterOp::Eq)
        .map(|filter| filter_value(filter, params))
        .transpose()
}

fn filter_bytes_for_column<'a>(
    filters: &'a [SqlFilter],
    params: &'a [SqlValue],
    column: &str,
) -> Result<Option<&'a [u8]>, DbError> {
    Ok(eq_filter_value_for_column(filters, params, column)?.and_then(SqlValue::as_bytes))
}

fn filter_u64_for_column(
    filters: &[SqlFilter],
    params: &[SqlValue],
    column: &str,
) -> Result<Option<u64>, DbError> {
    Ok(eq_filter_value_for_column(filters, params, column)?
        .and_then(SqlValue::as_i64)
        .and_then(|value| (value >= 0).then_some(value as u64)))
}

fn assignment_value<'a>(
    assignment: &'a SqlAssignment,
    params: &'a [SqlValue],
) -> Result<&'a SqlValue, DbError> {
    sql_value(&assignment.value, params)
}

fn non_negative_assignment_u64(value: &SqlValue, column: &str) -> Result<u64, DbError> {
    let value = value
        .as_i64()
        .ok_or_else(|| DbError::Sql(format!("{column} assignment requires integer")))?;
    if value < 0 {
        return Err(DbError::Sql(format!(
            "{column} assignment must be non-negative"
        )));
    }
    Ok(value as u64)
}

fn optional_text_assignment(value: &SqlValue, column: &str) -> Result<Option<String>, DbError> {
    match value {
        SqlValue::Null => Ok(None),
        SqlValue::Text(value) => Ok(Some(value.clone())),
        _ => Err(DbError::Sql(format!(
            "{column} assignment requires text or NULL"
        ))),
    }
}

fn sql_value<'a>(
    value: &'a SqlFilterValue,
    params: &'a [SqlValue],
) -> Result<&'a SqlValue, DbError> {
    match value {
        SqlFilterValue::Param(idx) => params
            .get(*idx)
            .ok_or_else(|| DbError::Sql(format!("missing SQL parameter {}", idx + 1))),
        SqlFilterValue::Literal(value) => Ok(value),
    }
}

fn require_single_filter(filters: &[SqlFilter]) -> Result<&SqlFilter, DbError> {
    if filters.len() != 1 {
        return Err(DbError::Sql(
            "UPDATE requires exactly one primary-key WHERE predicate".into(),
        ));
    }
    Ok(&filters[0])
}

fn require_filter_column(filter: &SqlFilter, column: &str) -> Result<(), DbError> {
    if filter.column == column && filter.op == SqlFilterOp::Eq {
        Ok(())
    } else {
        Err(DbError::Sql(format!(
            "statement requires WHERE {column} = ..."
        )))
    }
}

fn required_column<'a>(
    columns: &[String],
    params: &'a [SqlValue],
    column: &str,
) -> Result<&'a SqlValue, DbError> {
    let idx = columns
        .iter()
        .position(|candidate| candidate == column)
        .ok_or_else(|| DbError::Sql(format!("missing required column {column}")))?;
    params
        .get(idx)
        .ok_or_else(|| DbError::Sql(format!("missing value for column {column}")))
}

fn required_column_bytes<'a>(
    columns: &[String],
    params: &'a [SqlValue],
    column: &str,
) -> Result<&'a [u8], DbError> {
    required_column(columns, params, column)?
        .as_bytes()
        .ok_or_else(|| DbError::Sql(format!("column {column} requires bytes or text")))
}

fn required_column_str<'a>(
    columns: &[String],
    params: &'a [SqlValue],
    column: &str,
) -> Result<&'a str, DbError> {
    required_column(columns, params, column)?
        .as_str()
        .ok_or_else(|| DbError::Sql(format!("column {column} requires text")))
}

fn required_column_i64(
    columns: &[String],
    params: &[SqlValue],
    column: &str,
) -> Result<i64, DbError> {
    required_column(columns, params, column)?
        .as_i64()
        .ok_or_else(|| DbError::Sql(format!("column {column} requires integer")))
}

fn required_column_u64(
    columns: &[String],
    params: &[SqlValue],
    column: &str,
) -> Result<u64, DbError> {
    let value = required_column_i64(columns, params, column)?;
    if value < 0 {
        return Err(DbError::Sql(format!(
            "column {column} must be non-negative"
        )));
    }
    Ok(value as u64)
}

fn optional_column_str<'a>(
    columns: &[String],
    params: &'a [SqlValue],
    column: &str,
) -> Result<Option<&'a str>, DbError> {
    let Some(idx) = columns.iter().position(|candidate| candidate == column) else {
        return Ok(None);
    };
    match params
        .get(idx)
        .ok_or_else(|| DbError::Sql(format!("missing value for column {column}")))?
    {
        SqlValue::Null => Ok(None),
        SqlValue::Text(value) => Ok(Some(value)),
        _ => Err(DbError::Sql(format!(
            "column {column} requires text or NULL"
        ))),
    }
}

fn sql_write_result(rows_affected: usize) -> SqlResult {
    SqlResult {
        columns: Vec::new(),
        rows: Vec::new(),
        rows_affected,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemStorage {
    bytes: Vec<u8>,
}

impl MemStorage {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl DbStorage for MemStorage {
    fn len(&mut self) -> Result<u64, DbError> {
        Ok(self.bytes.len() as u64)
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), DbError> {
        let start = usize::try_from(offset)
            .map_err(|_| DbError::Storage("memory read offset too large".into()))?;
        let end = start.saturating_add(buf.len());
        if end > self.bytes.len() {
            return Err(DbError::Storage("memory read past end".into()));
        }
        buf.copy_from_slice(&self.bytes[start..end]);
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> Result<u64, DbError> {
        let offset = self.bytes.len() as u64;
        self.bytes.extend_from_slice(bytes);
        Ok(offset)
    }

    fn truncate(&mut self, len: u64) -> Result<(), DbError> {
        let len = usize::try_from(len)
            .map_err(|_| DbError::Storage("memory truncate length too large".into()))?;
        if len > self.bytes.len() {
            return Err(DbError::Storage("memory truncate length past end".into()));
        }
        self.bytes.truncate(len);
        Ok(())
    }
}

#[cfg(feature = "std")]
pub struct FileStorage {
    file: std::fs::File,
}

#[cfg(feature = "std")]
impl FileStorage {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, DbError> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .map_err(|e| DbError::Storage(e.to_string()))?;
        Ok(Self { file })
    }

    pub fn create_truncated(path: impl AsRef<std::path::Path>) -> Result<Self, DbError> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|e| DbError::Storage(e.to_string()))?;
        Ok(Self { file })
    }
}

#[cfg(feature = "std")]
impl DbStorage for FileStorage {
    fn len(&mut self) -> Result<u64, DbError> {
        self.file
            .metadata()
            .map(|metadata| metadata.len())
            .map_err(|e| DbError::Storage(e.to_string()))
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), DbError> {
        use std::io::{Read, Seek, SeekFrom};

        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| DbError::Storage(e.to_string()))?;
        self.file
            .read_exact(buf)
            .map_err(|e| DbError::Storage(e.to_string()))
    }

    fn append(&mut self, bytes: &[u8]) -> Result<u64, DbError> {
        use std::io::{Seek, SeekFrom, Write};

        let offset = self
            .file
            .seek(SeekFrom::End(0))
            .map_err(|e| DbError::Storage(e.to_string()))?;
        self.file
            .write_all(bytes)
            .map_err(|e| DbError::Storage(e.to_string()))?;
        Ok(offset)
    }

    fn sync(&mut self) -> Result<(), DbError> {
        self.file
            .sync_all()
            .map_err(|e| DbError::Storage(e.to_string()))
    }

    fn truncate(&mut self, len: u64) -> Result<(), DbError> {
        self.file
            .set_len(len)
            .map_err(|e| DbError::Storage(e.to_string()))
    }
}

#[cfg(feature = "std")]
pub fn open_file_database(
    path: impl AsRef<std::path::Path>,
) -> Result<DerivedDb<FileStorage>, DbError> {
    DerivedDb::open(FileStorage::open(path)?)
}

#[cfg(feature = "std")]
pub fn open_file_database_repairing_tail(
    path: impl AsRef<std::path::Path>,
) -> Result<DerivedDb<FileStorage>, DbError> {
    DerivedDb::open_repairing_tail(FileStorage::open(path)?)
}

#[cfg(feature = "std")]
pub fn compact_file_database(path: impl AsRef<std::path::Path>) -> Result<(), DbError> {
    let path = path.as_ref();
    let db = open_file_database(path)?;
    let tmp_path = compact_tmp_path(path);
    if let Some(parent) = tmp_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DbError::Storage(e.to_string()))?;
    }

    let compacted = db.compact_into(FileStorage::create_truncated(&tmp_path)?)?;
    let mut compacted_storage = compacted.into_storage();
    compacted_storage.sync()?;
    drop(compacted_storage);

    let verified = open_file_database(&tmp_path)?;
    drop(verified);

    std::fs::rename(&tmp_path, path).map_err(|e| DbError::Storage(e.to_string()))?;
    sync_parent_dir(path)?;
    Ok(())
}

#[cfg(feature = "std")]
fn compact_tmp_path(path: &std::path::Path) -> std::path::PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("derived-db");
    path.with_file_name(format!("{file_name}.compact.tmp"))
}

#[cfg(feature = "std")]
fn sync_parent_dir(path: &std::path::Path) -> Result<(), DbError> {
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let dir = std::fs::File::open(parent).map_err(|e| DbError::Storage(e.to_string()))?;
    dir.sync_all().map_err(|e| DbError::Storage(e.to_string()))
}

#[cfg(feature = "std")]
pub fn initialize_runtime_database(
    path: impl AsRef<std::path::Path>,
    node_label: &str,
    origin: &str,
    updated_at: i64,
) -> Result<(), DbError> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent).map_err(|e| DbError::Storage(e.to_string()))?;
    }
    let mut db = open_file_database(path)?;
    db.put_meta(b"node_label", node_label.as_bytes(), updated_at)?;
    db.put_meta(b"origin", origin.as_bytes(), updated_at)
}

fn require_non_empty(value: &[u8], name: &str) -> Result<(), DbError> {
    if value.is_empty() {
        return Err(DbError::InvalidArgument(format!(
            "{name} must not be empty"
        )));
    }
    Ok(())
}

fn validate_object_index_row(row: ObjectIndexRow<'_>) -> Result<(), DbError> {
    require_non_empty(row.object_id, "derived_object_index.object_id")?;
    require_non_empty(row.stream_id, "derived_object_index.stream_id")?;
    require_non_empty(
        row.representation_id,
        "derived_object_index.representation_id",
    )
}

fn file_header() -> [u8; FILE_HEADER_LEN as usize] {
    let mut header = [0u8; FILE_HEADER_LEN as usize];
    header[..MAGIC.len()].copy_from_slice(MAGIC);
    header[8..10].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
    header[10..12].copy_from_slice(&SCHEMA_VERSION.to_le_bytes());
    header[12..16].copy_from_slice(&HEADER_FLAGS.to_le_bytes());
    let checksum = header_checksum(&header[..16]);
    header[16..20].copy_from_slice(&checksum.to_le_bytes());
    header
}

fn validate_file_header<S: DbStorage>(storage: &mut S, len: u64) -> Result<(), DbError> {
    if len < FILE_HEADER_LEN {
        return Err(DbError::NeedsRebuild(
            "derived db header is missing version metadata".into(),
        ));
    }

    let mut header = [0u8; FILE_HEADER_LEN as usize];
    storage.read_exact_at(0, &mut header)?;
    if &header[..MAGIC.len()] != MAGIC {
        return Err(DbError::Decode("derived db magic mismatch".into()));
    }

    let expected_checksum = u32::from_le_bytes([header[16], header[17], header[18], header[19]]);
    let actual_checksum = header_checksum(&header[..16]);
    if expected_checksum != actual_checksum {
        return Err(DbError::Decode(
            "derived db header checksum mismatch".into(),
        ));
    }

    let format_version = u16::from_le_bytes([header[8], header[9]]);
    if format_version != FORMAT_VERSION {
        return Err(DbError::NeedsRebuild(format!(
            "derived db format version {format_version} is incompatible with current format {FORMAT_VERSION}"
        )));
    }

    let schema_version = u16::from_le_bytes([header[10], header[11]]);
    if schema_version != SCHEMA_VERSION {
        return Err(DbError::NeedsRebuild(format!(
            "derived db schema version {schema_version} is incompatible with current schema {SCHEMA_VERSION}"
        )));
    }

    let flags = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
    if flags != HEADER_FLAGS {
        return Err(DbError::Decode(format!(
            "unsupported derived db header flags {flags:#x}"
        )));
    }

    Ok(())
}

fn append_encoded_record(out: &mut Vec<u8>, kind: u8, payload: &[u8]) -> Result<(), DbError> {
    let len = u32::try_from(payload.len())
        .map_err(|_| DbError::InvalidArgument("record payload too large".into()))?;
    let checksum = record_checksum(kind, payload);
    out.reserve(RECORD_HEADER_LEN as usize + payload.len());
    out.push(kind);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&checksum.to_le_bytes());
    out.extend_from_slice(payload);
    Ok(())
}

fn rkyv_encode<T>(value: &T) -> Result<Vec<u8>, DbError>
where
    T: for<'a> edgerun_wire::Serialize<
        edgerun_wire::rancor::Strategy<
            edgerun_wire::ser::Serializer<
                edgerun_wire::util::AlignedVec,
                edgerun_wire::ser::allocator::ArenaHandle<'a>,
                edgerun_wire::ser::sharing::Share,
            >,
            edgerun_wire::WireError,
        >,
    >,
{
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(value)
        .map(|bytes| bytes.into_vec())
        .map_err(|_| DbError::Decode("derived db rkyv encode failed".into()))
}

fn encode_meta_payload(key: &[u8], value: &[u8], updated_at: i64) -> Result<Vec<u8>, DbError> {
    rkyv_encode(&MetaRecordWire {
        key: key.to_vec(),
        value: value.to_vec(),
        updated_at,
    })
}

fn decode_meta_payload(payload: &[u8]) -> Result<MetaRecordWire, DbError> {
    let owned = payload.to_vec();
    edgerun_wire::from_bytes::<MetaRecordWire, edgerun_wire::WireError>(&owned)
        .map_err(|_| DbError::Decode("derived db meta record is not rkyv".into()))
}

fn encode_key_payload(key: &[u8]) -> Result<Vec<u8>, DbError> {
    rkyv_encode(&KeyRecordWire { key: key.to_vec() })
}

fn decode_key_payload(payload: &[u8], label: &str) -> Result<KeyRecordWire, DbError> {
    let owned = payload.to_vec();
    edgerun_wire::from_bytes::<KeyRecordWire, edgerun_wire::WireError>(&owned)
        .map_err(|_| DbError::Decode(format!("derived db {label} record is not rkyv")))
}

fn encode_object_index_payload(row: ObjectIndexRow<'_>) -> Result<Vec<u8>, DbError> {
    rkyv_encode(&ObjectIndexRecordWire {
        object_id: row.object_id.to_vec(),
        stream_id: row.stream_id.to_vec(),
        seq: row.seq,
        representation_id: row.representation_id.to_vec(),
        content_type: row.content_type.map(ToString::to_string),
        updated_at: row.updated_at,
    })
}

fn decode_object_index_payload(payload: &[u8]) -> Result<ObjectIndexRecordWire, DbError> {
    let owned = payload.to_vec();
    edgerun_wire::from_bytes::<ObjectIndexRecordWire, edgerun_wire::WireError>(&owned)
        .map_err(|_| DbError::Decode("derived db object index record is not rkyv".into()))
}

fn encode_admin_audit_payload(
    id: u64,
    event_time: i64,
    subject: &str,
    action: &str,
    outcome: &str,
) -> Result<Vec<u8>, DbError> {
    rkyv_encode(&AdminAuditRecordWire {
        id,
        event_time,
        subject: subject.to_string(),
        action: action.to_string(),
        outcome: outcome.to_string(),
    })
}

fn decode_admin_audit_payload(payload: &[u8]) -> Result<AdminAuditRecordWire, DbError> {
    let owned = payload.to_vec();
    edgerun_wire::from_bytes::<AdminAuditRecordWire, edgerun_wire::WireError>(&owned)
        .map_err(|_| DbError::Decode("derived db admin audit record is not rkyv".into()))
}

fn encode_pending_record(
    record: &PendingRecord,
    next_admin_audit_id: &mut u64,
) -> Result<(u8, Vec<u8>), DbError> {
    match record {
        PendingRecord::PutMeta(row) => {
            let payload = encode_meta_payload(&row.key, &row.value, row.updated_at)?;
            Ok((RECORD_META, payload))
        }
        PendingRecord::DeleteMeta(key) => {
            let payload = encode_key_payload(key)?;
            Ok((RECORD_DELETE_META, payload))
        }
        PendingRecord::UpsertObjectIndex(row) => {
            let payload = encode_object_index_payload(ObjectIndexRow {
                object_id: &row.object_id,
                stream_id: &row.stream_id,
                seq: row.seq,
                representation_id: &row.representation_id,
                content_type: row.content_type.as_deref(),
                updated_at: row.updated_at,
            })?;
            Ok((RECORD_OBJECT_INDEX, payload))
        }
        PendingRecord::DeleteObjectIndex(object_id) => {
            let payload = encode_key_payload(object_id)?;
            Ok((RECORD_DELETE_OBJECT_INDEX, payload))
        }
        PendingRecord::AppendAdminAudit(row) => {
            let id = *next_admin_audit_id;
            *next_admin_audit_id = next_admin_audit_id.saturating_add(1);
            let payload = encode_admin_audit_payload(
                id,
                row.event_time,
                &row.subject,
                &row.action,
                &row.outcome,
            )?;
            Ok((RECORD_ADMIN_AUDIT, payload))
        }
    }
}

fn header_checksum(bytes: &[u8]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn record_checksum(kind: u8, payload: &[u8]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    hash ^= u32::from(kind);
    hash = hash.wrapping_mul(0x0100_0193);
    for byte in payload {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[cfg(feature = "std")]
    fn temp_db_path(name: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "edgerun-derived-db-{name}-{}-{unique}.edb",
            std::process::id()
        ))
    }

    #[test]
    fn meta_round_trips_through_storage() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.put_meta(b"node_label", b"alpha", 7).unwrap();
        let storage = db.into_storage();

        let reopened = DerivedDb::open(storage).unwrap();
        let row = reopened.get_meta(b"node_label").unwrap();
        assert_eq!(row.value, b"alpha");
        assert_eq!(row.updated_at, 7);
    }

    #[test]
    fn new_storage_writes_versioned_header() {
        let db = DerivedDb::open(MemStorage::new()).unwrap();
        let storage = db.into_storage();
        assert_eq!(&storage.as_bytes()[..MAGIC.len()], MAGIC);
        assert_eq!(storage.as_bytes().len(), FILE_HEADER_LEN as usize);
        assert_eq!(
            u16::from_le_bytes([storage.as_bytes()[8], storage.as_bytes()[9]]),
            FORMAT_VERSION
        );
        assert_eq!(
            u16::from_le_bytes([storage.as_bytes()[10], storage.as_bytes()[11]]),
            SCHEMA_VERSION
        );
    }

    #[test]
    fn object_index_upsert_replaces_projection() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.upsert_object_index(ObjectIndexRow {
            object_id: b"object",
            stream_id: b"stream",
            seq: 1,
            representation_id: b"repr-a",
            content_type: Some("text/plain"),
            updated_at: 1,
        })
        .unwrap();
        db.upsert_object_index(ObjectIndexRow {
            object_id: b"object",
            stream_id: b"stream",
            seq: 2,
            representation_id: b"repr-b",
            content_type: None,
            updated_at: 2,
        })
        .unwrap();

        let row = db.get_object_index(b"object").unwrap();
        assert_eq!(row.seq, 2);
        assert_eq!(row.representation_id, b"repr-b");
        assert_eq!(row.content_type, None);
    }

    #[test]
    fn stream_filters_find_matching_object_rows() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.upsert_object_index(ObjectIndexRow {
            object_id: b"object-a",
            stream_id: b"stream-a",
            seq: 7,
            representation_id: b"repr-a",
            content_type: None,
            updated_at: 1,
        })
        .unwrap();
        db.upsert_object_index(ObjectIndexRow {
            object_id: b"object-b",
            stream_id: b"stream-b",
            seq: 7,
            representation_id: b"repr-b",
            content_type: None,
            updated_at: 1,
        })
        .unwrap();

        assert_eq!(db.object_index_for_stream(b"stream-a").count(), 1);
        assert_eq!(db.object_index_for_stream_seq(b"stream-a", 7).count(), 1);
        assert_eq!(db.object_index_for_stream_seq(b"stream-a", 8).count(), 0);
    }

    #[test]
    fn batch_applies_multiple_projection_updates_with_one_replayable_append() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        let mut batch = DbBatch::new();
        batch
            .put_meta(b"runtime.node", b"alpha", 1)
            .unwrap()
            .put_meta(b"runtime.origin", b"example.test", 1)
            .unwrap()
            .upsert_object_index(ObjectIndexRow {
                object_id: b"object",
                stream_id: b"stream",
                seq: 9,
                representation_id: b"repr",
                content_type: Some("application/octet-stream"),
                updated_at: 1,
            })
            .unwrap()
            .append_admin_audit(AdminAuditRow {
                event_time: 2,
                subject: "root",
                action: "batch",
                outcome: "ok",
            })
            .unwrap();

        assert_eq!(batch.len(), 4);
        db.apply_batch(batch).unwrap();
        let reopened = DerivedDb::open(db.into_storage()).unwrap();

        assert_eq!(reopened.meta_rows_with_prefix(b"runtime.").count(), 2);
        assert_eq!(reopened.get_object_index(b"object").unwrap().seq, 9);
        assert_eq!(reopened.admin_audit().len(), 1);
        assert_eq!(reopened.next_admin_audit_id(), 2);
    }

    #[test]
    fn batch_assigns_monotonic_admin_audit_ids() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.append_admin_audit(AdminAuditRow {
            event_time: 1,
            subject: "root",
            action: "before",
            outcome: "ok",
        })
        .unwrap();

        let mut batch = DbBatch::new();
        batch
            .append_admin_audit(AdminAuditRow {
                event_time: 2,
                subject: "root",
                action: "first",
                outcome: "ok",
            })
            .unwrap()
            .append_admin_audit(AdminAuditRow {
                event_time: 3,
                subject: "root",
                action: "second",
                outcome: "ok",
            })
            .unwrap();

        db.apply_batch(batch).unwrap();
        let reopened = DerivedDb::open(db.into_storage()).unwrap();
        let ids: Vec<u64> = reopened.admin_audit().iter().map(|row| row.id).collect();
        assert_eq!(ids, [1, 2, 3]);
        assert_eq!(reopened.next_admin_audit_id(), 4);
    }

    #[test]
    fn sql_schema_describes_public_tables() {
        let db = DerivedDb::open(MemStorage::new()).unwrap();
        let schema = db.sql_schema();
        assert_eq!(schema.len(), 3);
        assert_eq!(schema[0].name, "runtime_meta");
        assert_eq!(schema[0].columns[0].name, "key");
        assert!(schema[0].columns[0].primary_key);
        assert!(!schema[0].columns[0].mutable);
        assert_eq!(schema[1].columns[4].name, "content_type");
        assert!(schema[1].columns[4].nullable);
        assert_eq!(schema[2].name, "admin_audit");
        assert!(schema[2].append_only);
    }

    #[test]
    fn sql_insert_select_and_delete_runtime_meta() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        let result = db
            .execute_sql(
                "INSERT INTO runtime_meta (key, value, updated_at) VALUES (?, ?, ?)",
                &[
                    SqlValue::Text("node_label".into()),
                    SqlValue::Bytes(b"alpha".to_vec()),
                    SqlValue::Integer(11),
                ],
            )
            .unwrap();
        assert_eq!(result.rows_affected, 1);

        let result = db
            .execute_sql(
                "SELECT key, value, updated_at FROM runtime_meta WHERE key = ?",
                &[SqlValue::Text("node_label".into())],
            )
            .unwrap();
        assert_eq!(result.columns, ["key", "value", "updated_at"]);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0][1], SqlValue::Bytes(b"alpha".to_vec()));

        let result = db
            .execute_sql(
                "DELETE FROM runtime_meta WHERE key = ?",
                &[SqlValue::Text("node_label".into())],
            )
            .unwrap();
        assert_eq!(result.rows_affected, 1);
        assert!(db.get_meta(b"node_label").is_none());
    }

    #[test]
    fn sql_batch_applies_writes_with_one_atomic_validation_pass() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        let insert_params = [
            SqlValue::Text("node_label".into()),
            SqlValue::Bytes(b"alpha".to_vec()),
            SqlValue::Integer(11),
        ];
        let update_params = [
            SqlValue::Bytes(b"beta".to_vec()),
            SqlValue::Integer(12),
            SqlValue::Text("node_label".into()),
        ];
        let audit_params = [
            SqlValue::Integer(13),
            SqlValue::Text("root".into()),
            SqlValue::Text("batch".into()),
            SqlValue::Text("ok".into()),
        ];

        let result = db
            .execute_sql_batch(&[
                (
                    "INSERT INTO runtime_meta (key, value, updated_at) VALUES (?, ?, ?)",
                    &insert_params,
                ),
                (
                    "UPDATE runtime_meta SET value = ?, updated_at = ? WHERE key = ?",
                    &update_params,
                ),
                (
                    "INSERT INTO admin_audit (event_time, subject, action, outcome) VALUES (?, ?, ?, ?)",
                    &audit_params,
                ),
            ])
            .unwrap();

        assert_eq!(result.rows_affected, 3);
        let row = db.get_meta(b"node_label").unwrap();
        assert_eq!(row.value, b"beta");
        assert_eq!(row.updated_at, 12);
        assert_eq!(db.admin_audit()[0].id, 1);
        assert_eq!(db.admin_audit()[0].action, "batch");
    }

    #[test]
    fn sql_batch_failure_leaves_projection_and_storage_untouched() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        let before_len = db.storage.len().unwrap();
        let insert_params = [
            SqlValue::Text("node_label".into()),
            SqlValue::Bytes(b"alpha".to_vec()),
            SqlValue::Integer(11),
        ];
        let bad_params = [
            SqlValue::Text("node_label".into()),
            SqlValue::Bytes(b"ignored".to_vec()),
            SqlValue::Integer(12),
        ];

        let err = db
            .execute_sql_batch(&[
                (
                    "INSERT INTO runtime_meta (key, value, updated_at) VALUES (?, ?, ?)",
                    &insert_params,
                ),
                (
                    "INSERT INTO runtime_meta (key, value, updated_at, bogus) VALUES (?, ?, ?)",
                    &bad_params,
                ),
            ])
            .unwrap_err();

        assert!(
            matches!(err, DbError::Sql(message) if message.contains("column/value count mismatch"))
        );
        assert!(db.get_meta(b"node_label").is_none());
        assert_eq!(db.storage.len().unwrap(), before_len);
    }

    #[test]
    fn sql_insert_rejects_unknown_and_duplicate_columns() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        let err = db
            .execute_sql(
                "INSERT INTO runtime_meta (key, value, updated_at, ignored) VALUES (?, ?, ?, ?)",
                &[
                    SqlValue::Text("node_label".into()),
                    SqlValue::Bytes(b"alpha".to_vec()),
                    SqlValue::Integer(11),
                    SqlValue::Text("drop-me".into()),
                ],
            )
            .unwrap_err();
        assert!(matches!(err, DbError::Sql(message) if message.contains("unknown column ignored")));

        let err = db
            .execute_sql(
                "INSERT INTO runtime_meta (key, value, value, updated_at) VALUES (?, ?, ?, ?)",
                &[
                    SqlValue::Text("node_label".into()),
                    SqlValue::Bytes(b"alpha".to_vec()),
                    SqlValue::Bytes(b"beta".to_vec()),
                    SqlValue::Integer(11),
                ],
            )
            .unwrap_err();
        assert!(matches!(err, DbError::Sql(message) if message.contains("duplicate column value")));
    }

    #[test]
    fn sql_object_index_filter_uses_stream_and_seq() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.execute_sql(
            "INSERT INTO derived_object_index (object_id, stream_id, seq, representation_id, content_type, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            &[
                SqlValue::Bytes(b"object".to_vec()),
                SqlValue::Bytes(b"stream".to_vec()),
                SqlValue::Integer(7),
                SqlValue::Bytes(b"repr".to_vec()),
                SqlValue::Text("application/octet-stream".into()),
                SqlValue::Integer(12),
            ],
        )
        .unwrap();
        db.execute_sql(
            "INSERT INTO derived_object_index (object_id, stream_id, seq, representation_id, content_type, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            &[
                SqlValue::Bytes(b"object-2".to_vec()),
                SqlValue::Bytes(b"stream".to_vec()),
                SqlValue::Integer(8),
                SqlValue::Bytes(b"repr-2".to_vec()),
                SqlValue::Null,
                SqlValue::Integer(13),
            ],
        )
        .unwrap();

        let result = db
            .execute_sql(
                "SELECT object_id, representation_id FROM derived_object_index WHERE stream_id = ?",
                &[SqlValue::Bytes(b"stream".to_vec())],
            )
            .unwrap();
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[0][0], SqlValue::Bytes(b"object".to_vec()));

        let result = db
            .execute_sql(
                "SELECT object_id FROM derived_object_index WHERE stream_id = ?1 AND seq = ?2 LIMIT 1",
                &[
                    SqlValue::Bytes(b"stream".to_vec()),
                    SqlValue::Integer(8),
                ],
        )
        .unwrap();
        assert_eq!(
            result.rows,
            vec![vec![SqlValue::Bytes(b"object-2".to_vec())]]
        );
    }

    #[test]
    fn sql_explain_reports_index_access_paths() {
        let db = DerivedDb::open(MemStorage::new()).unwrap();
        assert_eq!(
            db.explain_sql("SELECT value FROM runtime_meta WHERE key = ?")
                .unwrap()
                .access_path,
            SqlAccessPath::RuntimeMetaKey
        );
        assert_eq!(
            db.explain_sql(
                "SELECT object_id FROM derived_object_index WHERE stream_id = ? AND seq = ?"
            )
            .unwrap()
            .access_path,
            SqlAccessPath::ObjectIndexStreamSeq
        );
        assert_eq!(
            db.explain_sql("SELECT object_id FROM derived_object_index WHERE stream_id = ?")
                .unwrap()
                .access_path,
            SqlAccessPath::ObjectIndexStream
        );
        assert_eq!(
            db.explain_sql("SELECT action FROM admin_audit WHERE id = 1")
                .unwrap()
                .access_path,
            SqlAccessPath::AdminAuditId
        );
        assert_eq!(
            db.explain_sql("SELECT action FROM admin_audit WHERE action = 'start'")
                .unwrap()
                .access_path,
            SqlAccessPath::FullScan
        );
    }

    #[test]
    fn object_secondary_indexes_track_upsert_delete_and_replay() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.upsert_object_index(ObjectIndexRow {
            object_id: b"object",
            stream_id: b"stream-a",
            seq: 1,
            representation_id: b"repr-a",
            content_type: None,
            updated_at: 1,
        })
        .unwrap();
        db.upsert_object_index(ObjectIndexRow {
            object_id: b"object",
            stream_id: b"stream-b",
            seq: 2,
            representation_id: b"repr-b",
            content_type: None,
            updated_at: 2,
        })
        .unwrap();

        assert_eq!(db.object_index_for_stream(b"stream-a").count(), 0);
        assert_eq!(db.object_index_for_stream_seq(b"stream-a", 1).count(), 0);
        assert_eq!(db.object_index_for_stream(b"stream-b").count(), 1);
        assert_eq!(db.object_index_for_stream_seq(b"stream-b", 2).count(), 1);

        let reopened = DerivedDb::open(db.into_storage()).unwrap();
        assert_eq!(reopened.object_index_for_stream(b"stream-a").count(), 0);
        assert_eq!(reopened.object_index_for_stream(b"stream-b").count(), 1);
        assert_eq!(
            reopened.object_index_for_stream_seq(b"stream-b", 2).count(),
            1
        );
    }

    #[test]
    fn sql_count_star_counts_filtered_rows() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.execute_sql(
            "INSERT INTO runtime_meta (key, value, updated_at) VALUES (?, ?, ?)",
            &[
                SqlValue::Text("node_label".into()),
                SqlValue::Bytes(b"alpha".to_vec()),
                SqlValue::Integer(11),
            ],
        )
        .unwrap();
        db.execute_sql(
            "INSERT INTO runtime_meta (key, value, updated_at) VALUES (?, ?, ?)",
            &[
                SqlValue::Text("origin".into()),
                SqlValue::Bytes(b"local".to_vec()),
                SqlValue::Integer(11),
            ],
        )
        .unwrap();
        db.execute_sql(
            "INSERT INTO runtime_meta (key, value, updated_at) VALUES (?, ?, ?)",
            &[
                SqlValue::Text("epoch".into()),
                SqlValue::Bytes(b"next".to_vec()),
                SqlValue::Integer(12),
            ],
        )
        .unwrap();

        let result = db
            .execute_sql("SELECT COUNT(*) FROM runtime_meta", &[])
            .unwrap();
        assert_eq!(result.columns, ["count"]);
        assert_eq!(result.rows, vec![vec![SqlValue::Integer(3)]]);

        let result = db
            .execute_sql(
                "SELECT count(*) FROM runtime_meta WHERE updated_at = ?1",
                &[SqlValue::Integer(11)],
            )
            .unwrap();
        assert_eq!(result.rows, vec![vec![SqlValue::Integer(2)]]);
    }

    #[test]
    fn sql_count_star_supports_and_filters() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.execute_sql(
            "INSERT INTO derived_object_index (object_id, stream_id, seq, representation_id, content_type, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            &[
                SqlValue::Bytes(b"object".to_vec()),
                SqlValue::Bytes(b"stream".to_vec()),
                SqlValue::Integer(7),
                SqlValue::Bytes(b"repr".to_vec()),
                SqlValue::Text("application/octet-stream".into()),
                SqlValue::Integer(12),
            ],
        )
        .unwrap();
        db.execute_sql(
            "INSERT INTO derived_object_index (object_id, stream_id, seq, representation_id, content_type, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            &[
                SqlValue::Bytes(b"object-2".to_vec()),
                SqlValue::Bytes(b"stream".to_vec()),
                SqlValue::Integer(8),
                SqlValue::Bytes(b"repr-2".to_vec()),
                SqlValue::Null,
                SqlValue::Integer(13),
            ],
        )
        .unwrap();

        let result = db
            .execute_sql(
                "SELECT COUNT(*) FROM derived_object_index WHERE stream_id = ?1 AND seq = ?2",
                &[SqlValue::Bytes(b"stream".to_vec()), SqlValue::Integer(8)],
            )
            .unwrap();
        assert_eq!(result.rows, vec![vec![SqlValue::Integer(1)]]);
    }

    #[test]
    fn sql_select_order_by_applies_before_limit() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        for (object_id, seq) in [("object-a", 7), ("object-b", 9), ("object-c", 8)] {
            db.execute_sql(
                "INSERT INTO derived_object_index (object_id, stream_id, seq, representation_id, content_type, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                &[
                    SqlValue::Text(object_id.into()),
                    SqlValue::Bytes(b"stream".to_vec()),
                    SqlValue::Integer(seq),
                    SqlValue::Bytes(b"repr".to_vec()),
                    SqlValue::Null,
                    SqlValue::Integer(12),
                ],
            )
            .unwrap();
        }

        let result = db
            .execute_sql(
                "SELECT object_id, seq FROM derived_object_index WHERE stream_id = ? ORDER BY seq DESC LIMIT 2",
                &[SqlValue::Bytes(b"stream".to_vec())],
            )
            .unwrap();
        assert_eq!(
            result.rows,
            vec![
                vec![SqlValue::Bytes(b"object-b".to_vec()), SqlValue::Integer(9)],
                vec![SqlValue::Bytes(b"object-c".to_vec()), SqlValue::Integer(8)],
            ]
        );
    }

    #[test]
    fn sql_select_order_by_can_use_unselected_column() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.execute_sql(
            "INSERT INTO admin_audit (event_time, subject, action, outcome) VALUES (?, ?, ?, ?)",
            &[
                SqlValue::Integer(20),
                SqlValue::Text("root".into()),
                SqlValue::Text("second".into()),
                SqlValue::Text("ok".into()),
            ],
        )
        .unwrap();
        db.execute_sql(
            "INSERT INTO admin_audit (event_time, subject, action, outcome) VALUES (?, ?, ?, ?)",
            &[
                SqlValue::Integer(10),
                SqlValue::Text("root".into()),
                SqlValue::Text("first".into()),
                SqlValue::Text("ok".into()),
            ],
        )
        .unwrap();

        let result = db
            .execute_sql(
                "SELECT action FROM admin_audit ORDER BY event_time ASC",
                &[],
            )
            .unwrap();
        assert_eq!(
            result.rows,
            vec![
                vec![SqlValue::Text("first".into())],
                vec![SqlValue::Text("second".into())],
            ]
        );
    }

    #[test]
    fn sql_integer_range_filters_audit_rows() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        for (event_time, action) in [(10, "old"), (20, "mid"), (30, "new")] {
            db.execute_sql(
                "INSERT INTO admin_audit (event_time, subject, action, outcome) VALUES (?, ?, ?, ?)",
                &[
                    SqlValue::Integer(event_time),
                    SqlValue::Text("root".into()),
                    SqlValue::Text(action.into()),
                    SqlValue::Text("ok".into()),
                ],
            )
            .unwrap();
        }

        let result = db
            .execute_sql(
                "SELECT action FROM admin_audit WHERE event_time >= ?1 AND event_time < ?2 ORDER BY event_time ASC",
                &[SqlValue::Integer(20), SqlValue::Integer(30)],
            )
            .unwrap();
        assert_eq!(result.rows, vec![vec![SqlValue::Text("mid".into())]]);
    }

    #[test]
    fn sql_integer_range_filters_object_sequences() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        for seq in [1, 2, 3] {
            db.execute_sql(
                "INSERT INTO derived_object_index (object_id, stream_id, seq, representation_id, content_type, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                &[
                    SqlValue::Text(format!("object-{seq}")),
                    SqlValue::Bytes(b"stream".to_vec()),
                    SqlValue::Integer(seq),
                    SqlValue::Bytes(b"repr".to_vec()),
                    SqlValue::Null,
                    SqlValue::Integer(12),
                ],
            )
            .unwrap();
        }

        let result = db
            .execute_sql(
                "SELECT object_id FROM derived_object_index WHERE stream_id = ?1 AND seq > ?2 ORDER BY seq ASC",
                &[SqlValue::Bytes(b"stream".to_vec()), SqlValue::Integer(1)],
            )
            .unwrap();
        assert_eq!(
            result.rows,
            vec![
                vec![SqlValue::Bytes(b"object-2".to_vec())],
                vec![SqlValue::Bytes(b"object-3".to_vec())],
            ]
        );
        assert_eq!(
            db.explain_sql(
                "SELECT object_id FROM derived_object_index WHERE stream_id = ?1 AND seq > ?2"
            )
            .unwrap()
            .access_path,
            SqlAccessPath::ObjectIndexStream
        );
    }

    #[test]
    fn sql_range_rejects_text_and_blob_columns() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.execute_sql(
            "INSERT INTO admin_audit (event_time, subject, action, outcome) VALUES (?, ?, ?, ?)",
            &[
                SqlValue::Integer(10),
                SqlValue::Text("root".into()),
                SqlValue::Text("start".into()),
                SqlValue::Text("ok".into()),
            ],
        )
        .unwrap();

        let err = db
            .execute_sql("SELECT action FROM admin_audit WHERE subject > 'root'", &[])
            .unwrap_err();
        assert!(
            matches!(err, DbError::Sql(message) if message.contains("text columns only support equality"))
        );

        db.execute_sql(
            "INSERT INTO runtime_meta (key, value, updated_at) VALUES (?, ?, ?)",
            &[
                SqlValue::Text("node_label".into()),
                SqlValue::Bytes(b"alpha".to_vec()),
                SqlValue::Integer(11),
            ],
        )
        .unwrap();
        let err = db
            .execute_sql("SELECT key FROM runtime_meta WHERE key > 'node_label'", &[])
            .unwrap_err();
        assert!(
            matches!(err, DbError::Sql(message) if message.contains("bytes columns only support equality"))
        );
    }

    #[test]
    fn sql_quoted_literals_do_not_split_clauses() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.execute_sql(
            "INSERT INTO admin_audit (event_time, subject, action, outcome) VALUES (?, ?, ?, ?)",
            &[
                SqlValue::Integer(20),
                SqlValue::Text("root".into()),
                SqlValue::Text("deploy".into()),
                SqlValue::Text("ok WHERE nope ORDER BY nope LIMIT nope".into()),
            ],
        )
        .unwrap();

        let result = db
            .execute_sql(
                "SELECT action FROM admin_audit WHERE subject = 'root' AND outcome = 'ok WHERE nope ORDER BY nope LIMIT nope'",
                &[],
            )
            .unwrap();
        assert_eq!(result.rows, vec![vec![SqlValue::Text("deploy".into())]]);
    }

    #[test]
    fn sql_update_allows_commas_and_equals_in_quoted_values() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.execute_sql(
            "INSERT INTO runtime_meta (key, value, updated_at) VALUES (?, ?, ?)",
            &[
                SqlValue::Text("status".into()),
                SqlValue::Text("old".into()),
                SqlValue::Integer(1),
            ],
        )
        .unwrap();

        db.execute_sql(
            "UPDATE runtime_meta SET value = 'a,b=c', updated_at = 2 WHERE key = 'status'",
            &[],
        )
        .unwrap();
        assert_eq!(db.get_meta(b"status").unwrap().value, b"a,b=c");
        assert_eq!(db.get_meta(b"status").unwrap().updated_at, 2);
    }

    #[test]
    fn sql_update_runtime_meta_by_primary_key() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.execute_sql(
            "INSERT INTO runtime_meta (key, value, updated_at) VALUES (?, ?, ?)",
            &[
                SqlValue::Text("node_label".into()),
                SqlValue::Bytes(b"alpha".to_vec()),
                SqlValue::Integer(11),
            ],
        )
        .unwrap();

        let result = db
            .execute_sql(
                "UPDATE runtime_meta SET value = ?, updated_at = ? WHERE key = ?",
                &[
                    SqlValue::Bytes(b"beta".to_vec()),
                    SqlValue::Integer(12),
                    SqlValue::Text("node_label".into()),
                ],
            )
            .unwrap();
        assert_eq!(result.rows_affected, 1);

        let row = db.get_meta(b"node_label").unwrap();
        assert_eq!(row.value, b"beta");
        assert_eq!(row.updated_at, 12);
    }

    #[test]
    fn sql_update_rejects_immutable_or_duplicate_assignments() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.execute_sql(
            "INSERT INTO runtime_meta (key, value, updated_at) VALUES (?, ?, ?)",
            &[
                SqlValue::Text("node_label".into()),
                SqlValue::Bytes(b"alpha".to_vec()),
                SqlValue::Integer(11),
            ],
        )
        .unwrap();

        let err = db
            .execute_sql(
                "UPDATE runtime_meta SET key = 'new' WHERE key = 'node_label'",
                &[],
            )
            .unwrap_err();
        assert!(
            matches!(err, DbError::Sql(message) if message.contains("column key cannot be updated"))
        );

        let err = db
            .execute_sql(
                "UPDATE runtime_meta SET value = 'a', value = 'b' WHERE key = 'node_label'",
                &[],
            )
            .unwrap_err();
        assert!(matches!(err, DbError::Sql(message) if message.contains("duplicate column value")));
    }

    #[test]
    fn sql_update_object_index_and_replay() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.execute_sql(
            "INSERT INTO derived_object_index (object_id, stream_id, seq, representation_id, content_type, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            &[
                SqlValue::Bytes(b"object".to_vec()),
                SqlValue::Bytes(b"stream".to_vec()),
                SqlValue::Integer(7),
                SqlValue::Bytes(b"repr".to_vec()),
                SqlValue::Text("application/octet-stream".into()),
                SqlValue::Integer(12),
            ],
        )
        .unwrap();

        db.execute_sql(
            "UPDATE derived_object_index SET seq = ?1, content_type = NULL, updated_at = ?2 WHERE object_id = ?3",
            &[
                SqlValue::Integer(8),
                SqlValue::Integer(13),
                SqlValue::Bytes(b"object".to_vec()),
            ],
        )
        .unwrap();
        let reopened = DerivedDb::open(db.into_storage()).unwrap();
        let row = reopened.get_object_index(b"object").unwrap();
        assert_eq!(row.seq, 8);
        assert_eq!(row.content_type, None);
        assert_eq!(row.updated_at, 13);
    }

    #[test]
    fn sql_update_keeps_admin_audit_append_only() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.execute_sql(
            "INSERT INTO admin_audit (event_time, subject, action, outcome) VALUES (?, ?, ?, ?)",
            &[
                SqlValue::Integer(13),
                SqlValue::Text("root".into()),
                SqlValue::Text("start".into()),
                SqlValue::Text("ok".into()),
            ],
        )
        .unwrap();

        let err = db
            .execute_sql(
                "UPDATE admin_audit SET outcome = 'changed' WHERE id = 1",
                &[],
            )
            .unwrap_err();
        assert!(matches!(err, DbError::Sql(message) if message.contains("append-only")));
    }

    #[test]
    fn sql_admin_audit_is_append_only() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.execute_sql(
            "INSERT INTO admin_audit (event_time, subject, action, outcome) VALUES (?, ?, ?, ?)",
            &[
                SqlValue::Integer(13),
                SqlValue::Text("root".into()),
                SqlValue::Text("start".into()),
                SqlValue::Text("ok".into()),
            ],
        )
        .unwrap();

        let result = db
            .execute_sql(
                "SELECT id, action FROM admin_audit WHERE subject = 'root'",
                &[],
            )
            .unwrap();
        assert_eq!(
            result.rows,
            vec![vec![SqlValue::Integer(1), SqlValue::Text("start".into())]]
        );

        let err = db
            .execute_sql("DELETE FROM admin_audit WHERE id = 1", &[])
            .unwrap_err();
        assert!(matches!(err, DbError::Sql(message) if message.contains("append-only")));
    }

    #[test]
    fn deletes_survive_replay() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.put_meta(b"node_label", b"alpha", 1).unwrap();
        db.upsert_object_index(ObjectIndexRow {
            object_id: b"object",
            stream_id: b"stream",
            seq: 1,
            representation_id: b"repr",
            content_type: None,
            updated_at: 1,
        })
        .unwrap();

        assert!(db.delete_meta(b"node_label").unwrap());
        assert!(db.delete_object_index(b"object").unwrap());
        let storage = db.into_storage();
        let reopened = DerivedDb::open(storage).unwrap();

        assert!(reopened.get_meta(b"node_label").is_none());
        assert!(reopened.get_object_index(b"object").is_none());
        assert_eq!(reopened.meta_rows().count(), 0);
        assert_eq!(reopened.object_index_rows().count(), 0);
    }

    #[test]
    fn deleting_missing_rows_is_recorded_but_reports_absent() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        assert!(!db.delete_meta(b"missing").unwrap());
        assert!(!db.delete_object_index(b"missing").unwrap());

        let reopened = DerivedDb::open(db.into_storage()).unwrap();
        assert!(reopened.get_meta(b"missing").is_none());
        assert!(reopened.get_object_index(b"missing").is_none());
    }

    #[test]
    fn compaction_keeps_live_projection_and_drops_tombstones() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.put_meta(b"old", b"value", 1).unwrap();
        db.delete_meta(b"old").unwrap();
        db.put_meta(b"live", b"value", 2).unwrap();
        db.append_admin_audit(AdminAuditRow {
            event_time: 3,
            subject: "root",
            action: "compact",
            outcome: "ok",
        })
        .unwrap();

        let original_len = db.into_storage().as_bytes().len();
        let mut db = DerivedDb::open(MemStorage::from_bytes(Vec::new())).unwrap();
        db.put_meta(b"old", b"value", 1).unwrap();
        db.delete_meta(b"old").unwrap();
        db.put_meta(b"live", b"value", 2).unwrap();
        db.append_admin_audit(AdminAuditRow {
            event_time: 3,
            subject: "root",
            action: "compact",
            outcome: "ok",
        })
        .unwrap();
        let compacted = db.compact_into(MemStorage::new()).unwrap();
        let compacted_storage = compacted.into_storage();

        assert!(compacted_storage.as_bytes().len() < original_len);
        let reopened = DerivedDb::open(compacted_storage).unwrap();
        assert!(reopened.get_meta(b"old").is_none());
        assert_eq!(reopened.get_meta(b"live").unwrap().value, b"value");
        assert_eq!(reopened.admin_audit().len(), 1);
    }

    #[cfg(feature = "std")]
    #[test]
    fn file_compaction_replaces_database_with_verified_compact_copy() {
        let path = temp_db_path("compact-file");
        let tmp_path = compact_tmp_path(&path);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&tmp_path);

        {
            let mut db = open_file_database(&path).unwrap();
            db.put_meta(b"old", b"value", 1).unwrap();
            db.delete_meta(b"old").unwrap();
            db.put_meta(b"live", b"value", 2).unwrap();
            db.append_admin_audit(AdminAuditRow {
                event_time: 3,
                subject: "root",
                action: "compact",
                outcome: "ok",
            })
            .unwrap();
        }

        let original_len = std::fs::metadata(&path).unwrap().len();
        compact_file_database(&path).unwrap();
        let compacted_len = std::fs::metadata(&path).unwrap().len();
        assert!(compacted_len < original_len);
        assert!(!tmp_path.exists());

        let reopened = open_file_database(&path).unwrap();
        assert!(reopened.get_meta(b"old").is_none());
        assert_eq!(reopened.get_meta(b"live").unwrap().value, b"value");
        assert_eq!(reopened.admin_audit().len(), 1);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn admin_audit_ids_continue_after_reopen() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        assert_eq!(
            db.append_admin_audit(AdminAuditRow {
                event_time: 1,
                subject: "root",
                action: "start",
                outcome: "ok",
            })
            .unwrap(),
            1
        );
        let storage = db.into_storage();
        let mut reopened = DerivedDb::open(storage).unwrap();
        assert_eq!(
            reopened
                .append_admin_audit(AdminAuditRow {
                    event_time: 2,
                    subject: "root",
                    action: "stop",
                    outcome: "ok",
                })
                .unwrap(),
            2
        );
    }

    #[test]
    fn open_reports_needs_rebuild_for_missing_header_version() {
        let bytes = MAGIC.to_vec();
        let err = match DerivedDb::open(MemStorage::from_bytes(bytes)) {
            Ok(_) => panic!("missing header version should fail"),
            Err(err) => err,
        };
        assert!(
            matches!(err, DbError::NeedsRebuild(message) if message.contains("header is missing"))
        );
    }

    #[test]
    fn open_reports_needs_rebuild_for_incompatible_schema_version() {
        let mut bytes = file_header().to_vec();
        bytes[10..12].copy_from_slice(&SCHEMA_VERSION.saturating_add(1).to_le_bytes());
        let checksum = header_checksum(&bytes[..16]);
        bytes[16..20].copy_from_slice(&checksum.to_le_bytes());

        let err = match DerivedDb::open(MemStorage::from_bytes(bytes)) {
            Ok(_) => panic!("incompatible schema should fail"),
            Err(err) => err,
        };
        assert!(
            matches!(err, DbError::NeedsRebuild(message) if message.contains("schema version"))
        );
    }

    #[test]
    fn open_rejects_corrupt_header_checksum() {
        let mut bytes = file_header().to_vec();
        bytes[19] ^= 0x55;

        let err = match DerivedDb::open(MemStorage::from_bytes(bytes)) {
            Ok(_) => panic!("corrupt header checksum should fail"),
            Err(err) => err,
        };
        assert!(matches!(err, DbError::Decode(message) if message.contains("header checksum")));
    }

    #[test]
    fn replay_rejects_truncated_record_payload() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.put_meta(b"node_label", b"alpha", 7).unwrap();
        let mut bytes = db.into_storage().as_bytes().to_vec();
        bytes.pop();

        let err = match DerivedDb::open(MemStorage::from_bytes(bytes)) {
            Ok(_) => panic!("truncated record should fail"),
            Err(err) => err,
        };
        assert!(matches!(err, DbError::Decode(message) if message.contains("truncated")));
    }

    #[test]
    fn repairing_tail_truncates_partial_record_header() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.put_meta(b"live", b"value", 1).unwrap();
        let mut storage = db.into_storage();
        storage.append(&[RECORD_META, 0, 0]).unwrap();
        let original_len = storage.as_bytes().len();

        let repaired = DerivedDb::open_repairing_tail(storage).unwrap();
        assert_eq!(repaired.get_meta(b"live").unwrap().value, b"value");
        let repaired_storage = repaired.into_storage();
        assert!(repaired_storage.as_bytes().len() < original_len);
        assert_eq!(
            DerivedDb::open(MemStorage::from_bytes(repaired_storage.as_bytes().to_vec()))
                .unwrap()
                .get_meta(b"live")
                .unwrap()
                .value,
            b"value"
        );
    }

    #[test]
    fn repairing_tail_truncates_partial_record_payload() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.put_meta(b"live", b"value", 1).unwrap();
        let mut storage = db.into_storage();
        let mut record = Vec::new();
        let payload = encode_meta_payload(b"partial", b"value", 2).unwrap();
        append_encoded_record(&mut record, RECORD_META, &payload).unwrap();
        storage.append(&record[..record.len() - 1]).unwrap();

        let repaired = DerivedDb::open_repairing_tail(storage).unwrap();
        assert_eq!(repaired.get_meta(b"live").unwrap().value, b"value");
        assert!(repaired.get_meta(b"partial").is_none());
    }

    #[test]
    fn repairing_tail_still_rejects_checksum_mismatch() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.put_meta(b"node_label", b"alpha", 7).unwrap();
        let mut bytes = db.into_storage().as_bytes().to_vec();
        let last = bytes.last_mut().unwrap();
        *last ^= 0x55;

        let err = match DerivedDb::open_repairing_tail(MemStorage::from_bytes(bytes)) {
            Ok(_) => panic!("checksum mismatch should not be repaired"),
            Err(err) => err,
        };
        assert!(matches!(err, DbError::Decode(message) if message.contains("checksum")));
    }

    #[test]
    fn replay_rejects_checksum_mismatch() {
        let mut db = DerivedDb::open(MemStorage::new()).unwrap();
        db.put_meta(b"node_label", b"alpha", 7).unwrap();
        let mut bytes = db.into_storage().as_bytes().to_vec();
        let last = bytes.last_mut().unwrap();
        *last ^= 0x55;

        let err = match DerivedDb::open(MemStorage::from_bytes(bytes)) {
            Ok(_) => panic!("checksum mismatch should fail"),
            Err(err) => err,
        };
        assert!(matches!(err, DbError::Decode(message) if message.contains("checksum")));
    }
}
