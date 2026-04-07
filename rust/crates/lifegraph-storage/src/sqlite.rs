//! SQLite-backed indexes for the Lifegraph storage layer.
//!
//! Implements the index layer from the protocol spec (§6.3, §19.9):
//! - Stream head index
//! - Stream seq/hash/offset lookup
//! - Replay cache (command deduplication)
//! - Fetch queue and missing-dependency tracking
//!
//! All indexes are rebuildable from the event log + encrypted blobs.
//! Loss of the SQLite database does not invalidate stored records.

use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;

/// Configuration for the SQLite index.
#[derive(Clone, Debug)]
pub struct SqliteIndexConfig {
    /// Path to the SQLite database file.
    pub db_path: PathBuf,
}

/// SQLite-backed index.
pub struct SqliteIndex {
    conn: Connection,
}

impl SqliteIndex {
    /// Opens an existing index or creates a new one with the required schema.
    pub fn open(config: &SqliteIndexConfig) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(&config.db_path)?;

        // Enable WAL mode for concurrent read/write
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )?;

        // Create tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS stream_heads (
                stream_id TEXT PRIMARY KEY NOT NULL,
                head_seq INTEGER NOT NULL,
                head_hash BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS events (
                stream_id TEXT NOT NULL,
                seq INTEGER NOT NULL,
                event_hash BLOB NOT NULL,
                file_offset INTEGER NOT NULL,
                envelope_version INTEGER NOT NULL,
                PRIMARY KEY (stream_id, seq)
            );

            CREATE TABLE IF NOT EXISTS replay_cache (
                target_node TEXT NOT NULL,
                command_hash TEXT NOT NULL,
                command_id TEXT NOT NULL,
                decision_event_seq INTEGER NOT NULL,
                PRIMARY KEY (target_node, command_hash)
            );

            CREATE TABLE IF NOT EXISTS fetch_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                target_type TEXT NOT NULL,   -- 'event', 'object', 'snapshot'
                target_id TEXT NOT NULL,
                priority INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending'  -- 'pending', 'in_progress', 'done', 'failed'
            );

            CREATE TABLE IF NOT EXISTS object_presence (
                object_id TEXT PRIMARY KEY NOT NULL,
                representation_id TEXT,
                blob_id TEXT,
                status TEXT NOT NULL DEFAULT 'missing'  -- 'missing', 'present', 'corrupted'
            );

            CREATE TABLE IF NOT EXISTS peers (
                node_id_hex TEXT PRIMARY KEY NOT NULL,
                addr TEXT,
                status TEXT NOT NULL DEFAULT 'unknown',  -- 'unknown', 'reachable', 'unreachable'
                last_seen INTEGER,
                first_seen INTEGER NOT NULL,
                is_bootstrap INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS snapshots (
                snapshot_id TEXT PRIMARY KEY NOT NULL,
                object_id_hex TEXT NOT NULL,
                view_type TEXT NOT NULL,
                producer_hex TEXT NOT NULL,
                produced_at INTEGER NOT NULL,
                completeness INTEGER NOT NULL,
                base_heads TEXT NOT NULL,
                stored_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS controller_changes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                controller_hex TEXT NOT NULL,
                change_type TEXT NOT NULL,  -- 'added', 'removed', 'transferred'
                event_seq INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS delegations (
                delegation_id TEXT PRIMARY KEY NOT NULL,
                issuer_hex TEXT NOT NULL,
                recipient_hex TEXT NOT NULL,
                capability_hex TEXT NOT NULL,
                expires_at INTEGER,
                is_revoked INTEGER NOT NULL DEFAULT 0,
                stored_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS revocations (
                revocation_id TEXT PRIMARY KEY NOT NULL,
                issuer_hex TEXT NOT NULL,
                target_type TEXT NOT NULL,  -- 'delegation', 'identity', 'node', 'object'
                target_hex TEXT NOT NULL,
                effective_at INTEGER,
                stored_at INTEGER NOT NULL
            );",
        )?;

        Ok(Self { conn })
    }

    // -----------------------------------------------------------------------
    // Event index
    // -----------------------------------------------------------------------

    /// Records an event in the index.
    pub fn put_event(
        &self,
        stream_id: &str,
        seq: i64,
        event_hash: &[u8],
        file_offset: u64,
        envelope_version: i64,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO events (stream_id, seq, event_hash, file_offset, envelope_version)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![stream_id, seq, event_hash, file_offset as i64, envelope_version],
        )?;
        Ok(())
    }

    /// Looks up an event by stream ID and sequence number.
    /// Returns (event_hash, file_offset) if found.
    pub fn get_event(
        &self,
        stream_id: &str,
        seq: i64,
    ) -> Result<Option<EventIndexEntry>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT event_hash, file_offset FROM events WHERE stream_id = ?1 AND seq = ?2",
                params![stream_id, seq],
                |row| {
                    let hash: Vec<u8> = row.get(0)?;
                    let offset: i64 = row.get(1)?;
                    Ok(EventIndexEntry {
                        event_hash: hash,
                        offset: offset as u64,
                    })
                },
            )
            .optional()
    }

    // -----------------------------------------------------------------------
    // Stream heads
    // -----------------------------------------------------------------------

    /// Updates the head for a stream.
    pub fn set_head(
        &self,
        stream_id: &str,
        seq: i64,
        hash: &[u8],
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO stream_heads (stream_id, head_seq, head_hash)
             VALUES (?1, ?2, ?3)",
            params![stream_id, seq, hash],
        )?;
        Ok(())
    }

    /// Returns the current head (seq, hash) for a stream.
    pub fn get_head(&self, stream_id: &str) -> Result<Option<(i64, Vec<u8>)>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT head_seq, head_hash FROM stream_heads WHERE stream_id = ?1",
                params![stream_id],
                |row| {
                    let seq: i64 = row.get(0)?;
                    let hash: Vec<u8> = row.get(1)?;
                    Ok((seq, hash))
                },
            )
            .optional()
    }

    // -----------------------------------------------------------------------
    // Replay cache (§19.10)
    // -----------------------------------------------------------------------

    /// Records a command's outcome for replay detection.
    /// The entry is keyed by command_hash (globally unique), with command_id
    /// stored as an idempotency hint.
    pub fn put_replay_entry(
        &self,
        target_node: &str,
        command_hash: &str,
        command_id: &str,
        decision_event_seq: i64,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO replay_cache (target_node, command_hash, command_id, decision_event_seq)
             VALUES (?1, ?2, ?3, ?4)",
            params![target_node, command_hash, command_id, decision_event_seq],
        )?;
        Ok(())
    }

    /// Checks if a command (by hash) has already been processed.
    pub fn get_replay_entry(
        &self,
        target_node: &str,
        command_hash: &str,
    ) -> Result<Option<ReplayEntry>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT command_id, decision_event_seq FROM replay_cache
                 WHERE target_node = ?1 AND command_hash = ?2",
                params![target_node, command_hash],
                |row| {
                    let command_id: String = row.get(0)?;
                    let seq: i64 = row.get(1)?;
                    Ok(ReplayEntry {
                        command_id,
                        decision_event_seq: seq,
                    })
                },
            )
            .optional()
    }

    // -----------------------------------------------------------------------
    // Fetch queue
    // -----------------------------------------------------------------------

    /// Enqueues a fetch request for a missing event, object, or snapshot.
    pub fn enqueue_fetch(
        &self,
        target_type: &str,
        target_id: &str,
        priority: i64,
    ) -> Result<(), rusqlite::Error> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO fetch_queue (target_type, target_id, priority, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![target_type, target_id, priority, now],
        )?;
        Ok(())
    }

    /// Dequeues the highest-priority pending fetch request.
    pub fn dequeue_fetch(&self) -> Result<Option<FetchEntry>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT id, target_type, target_id, priority FROM fetch_queue
                 WHERE status = 'pending'
                 ORDER BY priority DESC, created_at ASC
                 LIMIT 1",
                params![],
                |row| {
                    let id: i64 = row.get(0)?;
                    let target_type: String = row.get(1)?;
                    let target_id: String = row.get(2)?;
                    let priority: i64 = row.get(3)?;
                    Ok(FetchEntry {
                        id,
                        target_type,
                        target_id,
                        priority,
                    })
                },
            )
            .optional()
    }

    /// Marks a fetch entry as done.
    pub fn mark_fetch_done(&self, fetch_id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE fetch_queue SET status = 'done' WHERE id = ?1",
            params![fetch_id],
        )?;
        Ok(())
    }

    /// Marks a fetch entry as failed.
    pub fn mark_fetch_failed(&self, fetch_id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE fetch_queue SET status = 'failed' WHERE id = ?1",
            params![fetch_id],
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Object presence index
    // -----------------------------------------------------------------------

    /// Records that an object's representation is available.
    pub fn mark_object_present(
        &self,
        object_id: &str,
        representation_id: &str,
        blob_id: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO object_presence (object_id, representation_id, blob_id, status)
             VALUES (?1, ?2, ?3, 'present')",
            params![object_id, representation_id, blob_id],
        )?;
        Ok(())
    }

    /// Checks if an object is available.
    pub fn is_object_present(&self, object_id: &str) -> Result<bool, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT 1 FROM object_presence WHERE object_id = ?1 AND status = 'present'",
                params![object_id],
                |_| Ok(()),
            )
            .optional()
            .map(|r| r.is_some())
    }

    // -----------------------------------------------------------------------
    // Query support (§14.20–§14.21)
    // -----------------------------------------------------------------------

    /// Returns all known stream heads.
    pub fn list_stream_heads(&self) -> Result<Vec<(String, i64, Vec<u8>)>, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT stream_id, head_seq, head_hash FROM stream_heads")?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let seq: i64 = row.get(1)?;
            let hash: Vec<u8> = row.get(2)?;
            Ok((id, seq, hash))
        })?;
        rows.collect()
    }

    /// Returns events in the given stream and sequence range.
    /// Returns (seq, event_hash, envelope_version) for each event.
    pub fn list_event_range(
        &self,
        stream_id: &str,
        from_seq: i64,
        to_seq: i64,
    ) -> Result<Vec<(i64, Vec<u8>, i64)>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT seq, event_hash, envelope_version FROM events
             WHERE stream_id = ?1 AND seq >= ?2 AND seq <= ?3
             ORDER BY seq ASC",
        )?;
        let rows = stmt.query_map(
            params![stream_id, from_seq, to_seq],
            |row| {
                let seq: i64 = row.get(0)?;
                let hash: Vec<u8> = row.get(1)?;
                let ver: i64 = row.get(2)?;
                Ok((seq, hash, ver))
            },
        )?;
        rows.collect()
    }

    /// Returns all known event streams (distinct stream_ids in the events table).
    pub fn list_stream_ids(&self) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT stream_id FROM events ORDER BY stream_id")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect()
    }

    /// Returns object presence info for the given object IDs.
    /// Returns (object_id, representation_id, blob_id, status) for each found entry.
    pub fn lookup_objects(
        &self,
        object_ids: &[String],
    ) -> Result<Vec<(String, Option<String>, Option<String>, String)>, rusqlite::Error> {
        if object_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: String = object_ids.iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT object_id, representation_id, blob_id, status FROM object_presence
             WHERE object_id IN ({})",
            placeholders
        );
        let params: Vec<&dyn rusqlite::types::ToSql> = object_ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.into_iter()), |row| {
            let oid: String = row.get(0)?;
            let rid: Option<String> = row.get(1)?;
            let bid: Option<String> = row.get(2)?;
            let status: String = row.get(3)?;
            Ok((oid, rid, bid, status))
        })?;
        rows.collect()
    }

    // -----------------------------------------------------------------------
    // Peer database
    // -----------------------------------------------------------------------

    /// Upserts a known peer. If it already exists, updates addr, status, and last_seen.
    pub fn upsert_peer(
        &self,
        node_id_hex: &str,
        addr: Option<&str>,
        status: &str,
        is_bootstrap: bool,
    ) -> Result<(), rusqlite::Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        self.conn.execute(
            "INSERT INTO peers (node_id_hex, addr, status, last_seen, first_seen, is_bootstrap)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(node_id_hex) DO UPDATE SET
                 addr = COALESCE(excluded.addr, peers.addr),
                 status = excluded.status,
                 last_seen = excluded.last_seen",
            params![
                node_id_hex,
                addr,
                status,
                now,
                now,
                if is_bootstrap { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    /// Updates the reachability status of a peer.
    pub fn update_peer_status(
        &self,
        node_id_hex: &str,
        status: &str,
    ) -> Result<(), rusqlite::Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        self.conn.execute(
            "UPDATE peers SET status = ?1, last_seen = ?2 WHERE node_id_hex = ?3",
            params![status, now, node_id_hex],
        )?;
        Ok(())
    }

    /// Returns all known peers.
    pub fn list_peers(&self) -> Result<Vec<(String, Option<String>, String, Option<i64>, bool)>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT node_id_hex, addr, status, last_seen, is_bootstrap FROM peers ORDER BY last_seen DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            let nid: String = row.get(0)?;
            let addr: Option<String> = row.get(1)?;
            let status: String = row.get(2)?;
            let last_seen: Option<i64> = row.get(3)?;
            let is_boot: i64 = row.get(4)?;
            Ok((nid, addr, status, last_seen, is_boot != 0))
        })?;
        rows.collect()
    }

    /// Returns unreachable peers that have a known address, for reconnection attempts.
    pub fn list_unreachable_peers_with_addr(
        &self,
    ) -> Result<Vec<(String, String)>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT node_id_hex, addr FROM peers
             WHERE status = 'unreachable' AND addr IS NOT NULL
             ORDER BY last_seen ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            let nid: String = row.get(0)?;
            let addr: String = row.get(1)?;
            Ok((nid, addr))
        })?;
        rows.collect()
    }

    // -----------------------------------------------------------------------
    // Snapshots
    // -----------------------------------------------------------------------

    /// Stores a snapshot record.
    pub fn put_snapshot(
        &self,
        snapshot_id: &str,
        object_id_hex: &str,
        view_type: &str,
        producer_hex: &str,
        produced_at: i64,
        completeness: i32,
        base_heads: &str,
    ) -> Result<(), rusqlite::Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        self.conn.execute(
            "INSERT OR REPLACE INTO snapshots
             (snapshot_id, object_id_hex, view_type, producer_hex, produced_at, completeness, base_heads, stored_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![snapshot_id, object_id_hex, view_type, producer_hex, produced_at, completeness, base_heads, now],
        )?;
        Ok(())
    }

    /// Returns all known snapshots.
    pub fn list_snapshots(&self) -> Result<Vec<(String, String, String, String, i64, i32, String)>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT snapshot_id, object_id_hex, view_type, producer_hex, produced_at, completeness, base_heads
             FROM snapshots ORDER BY produced_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            let sid: String = row.get(0)?;
            let oid: String = row.get(1)?;
            let vt: String = row.get(2)?;
            let ph: String = row.get(3)?;
            let pa: i64 = row.get(4)?;
            let c: i32 = row.get(5)?;
            let bh: String = row.get(6)?;
            Ok((sid, oid, vt, ph, pa, c, bh))
        })?;
        rows.collect()
    }

    /// Returns a snapshot by ID.
    pub fn get_snapshot(&self, snapshot_id: &str) -> Result<Option<(String, String, String, String, i64, i32, String)>, rusqlite::Error> {
        self.conn.query_row(
            "SELECT snapshot_id, object_id_hex, view_type, producer_hex, produced_at, completeness, base_heads
             FROM snapshots WHERE snapshot_id = ?1",
            params![snapshot_id],
            |row| {
                let sid: String = row.get(0)?;
                let oid: String = row.get(1)?;
                let vt: String = row.get(2)?;
                let ph: String = row.get(3)?;
                let pa: i64 = row.get(4)?;
                let c: i32 = row.get(5)?;
                let bh: String = row.get(6)?;
                Ok((sid, oid, vt, ph, pa, c, bh))
            },
        ).optional()
    }

    // -----------------------------------------------------------------------
    // Controller state persistence
    // -----------------------------------------------------------------------

    /// Records a controller change (add/remove/transfer).
    pub fn record_controller_change(
        &self,
        controller_hex: &str,
        change_type: &str,
        event_seq: i64,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO controller_changes (controller_hex, change_type, event_seq)
             VALUES (?1, ?2, ?3)",
            params![controller_hex, change_type, event_seq],
        )?;
        Ok(())
    }

    /// Returns all controller changes up to and including the given event sequence number.
    /// Ordered by id (insertion order).
    pub fn list_controller_changes(
        &self,
        up_to_seq: i64,
    ) -> Result<Vec<(String, String)>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT controller_hex, change_type FROM controller_changes
             WHERE event_seq <= ?1 ORDER BY id ASC"
        )?;
        let rows = stmt.query_map(params![up_to_seq], |row| {
            let hex: String = row.get(0)?;
            let typ: String = row.get(1)?;
            Ok((hex, typ))
        })?;
        rows.collect()
    }

    // -----------------------------------------------------------------------
    // Delegation and revocation persistence
    // -----------------------------------------------------------------------

    /// Stores or updates a delegation record.
    pub fn store_delegation(
        &self,
        delegation_id: &str,
        issuer_hex: &str,
        recipient_hex: &str,
        capability_hex: &str,
        expires_at: Option<i64>,
    ) -> Result<(), rusqlite::Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        self.conn.execute(
            "INSERT OR REPLACE INTO delegations
             (delegation_id, issuer_hex, recipient_hex, capability_hex, expires_at, is_revoked, stored_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
            params![delegation_id, issuer_hex, recipient_hex, capability_hex, expires_at, now],
        )?;
        Ok(())
    }

    /// Marks a delegation as revoked.
    pub fn revoke_delegation(&self, delegation_id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE delegations SET is_revoked = 1 WHERE delegation_id = ?1",
            params![delegation_id],
        )?;
        Ok(())
    }

    /// Returns all non-revoked delegation IDs issued by the given issuer.
    pub fn list_active_delegations(&self, issuer_hex: &str) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT delegation_id FROM delegations
             WHERE issuer_hex = ?1 AND is_revoked = 0
             ORDER BY stored_at DESC"
        )?;
        let rows = stmt.query_map(params![issuer_hex], |row| row.get(0))?;
        rows.collect()
    }

    /// Returns all active revocation target IDs.
    pub fn list_active_revocations(&self) -> Result<Vec<(String, String)>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT target_type, target_hex FROM revocations ORDER BY stored_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            let typ: String = row.get(0)?;
            let hex: String = row.get(1)?;
            Ok((typ, hex))
        })?;
        rows.collect()
    }

    /// Records a revocation.
    pub fn store_revocation(
        &self,
        revocation_id: &str,
        issuer_hex: &str,
        target_type: &str,
        target_hex: &str,
        effective_at: Option<i64>,
    ) -> Result<(), rusqlite::Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        self.conn.execute(
            "INSERT OR REPLACE INTO revocations
             (revocation_id, issuer_hex, target_type, target_hex, effective_at, stored_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![revocation_id, issuer_hex, target_type, target_hex, effective_at, now],
        )?;
        // If this revocation targets a delegation, also mark it in the delegations table
        if target_type == "delegation" {
            self.revoke_delegation(target_hex)?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Maintenance
    // -----------------------------------------------------------------------

    /// Clears all index data (for rebuild). Does NOT touch event log files or blobs.
    pub fn clear(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(
            "DELETE FROM stream_heads;
             DELETE FROM events;
             DELETE FROM replay_cache;
             DELETE FROM fetch_queue;
             DELETE FROM object_presence;
             DELETE FROM peers;
             DELETE FROM snapshots;",
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Integrity and resilience
    // -----------------------------------------------------------------------

    /// Runs `PRAGMA integrity_check` and returns the result.
    /// Returns `Ok(())` if the database is intact, or `Err` with details.
    pub fn integrity_check(&self) -> Result<(), String> {
        let result: String = self.conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|e| format!("integrity_check query failed: {}", e))?;
        if result == "ok" {
            Ok(())
        } else {
            Err(result)
        }
    }

    /// Runs `PRAGMA wal_checkpoint(TRUNCATE)` to checkpoint and truncate the WAL file.
    /// Returns the checkpoint result code, or error.
    pub fn wal_checkpoint(&self) -> Result<i64, rusqlite::Error> {
        self.conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
    }

    /// Returns the current WAL file size in bytes, or None if the file doesn't exist.
    pub fn wal_size_bytes(&self) -> Result<Option<u64>, std::io::Error> {
        let wal_path = format!("{}-wal", self.conn.path().map(|p| p.to_string()).unwrap_or_default());
        let path = std::path::Path::new(&wal_path);
        if path.exists() {
            Ok(Some(path.metadata()?.len()))
        } else {
            Ok(None)
        }
    }
}

/// An entry in the event index.
pub struct EventIndexEntry {
    pub event_hash: Vec<u8>,
    pub offset: u64,
}

/// A replay cache entry.
pub struct ReplayEntry {
    pub command_id: String,
    pub decision_event_seq: i64,
}

/// A fetch queue entry.
pub struct FetchEntry {
    pub id: i64,
    pub target_type: String,
    pub target_id: String,
    pub priority: i64,
}
