//! SQLite-backed semantic cache for LLM responses.
//!
//! Uses embedding-based similarity search to find cached responses
//! for semantically similar queries.
//!
//! Uses r2d2 connection pooling for proper multi-threaded access.

use crate::embeddings::{cosine_similarity, EmbeddingModel};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::Path;
use std::sync::Arc;

/// Cache entry storing LLM request/response with embeddings
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub id: i64,
    pub query: String,
    pub response: String,
    pub embedding: Vec<f32>,
    pub timestamp: i64,
    pub hit_count: i64,
    pub metadata: Option<String>,
}

/// Semantic cache with SQLite storage and embedding-based retrieval
pub struct SemanticCache {
    db: Pool<SqliteConnectionManager>,
    embedding_model: Arc<EmbeddingModel>,
    dimension: usize,
    similarity_threshold: f32,
}

impl SemanticCache {
    /// Create or open cache database with connection pooling
    pub fn new(db_path: &Path, model: Arc<EmbeddingModel>) -> Result<Self, String> {
        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::builder()
            .max_size(10) // Allow up to 10 concurrent connections
            .build(manager)
            .map_err(|e| format!("Failed to create connection pool: {}", e))?;

        let dimension = model.dimension();

        // Initialize schema
        Self::init_schema(&pool, dimension)?;

        Ok(Self {
            db: pool,
            embedding_model: model,
            dimension,
            similarity_threshold: 0.85, // High threshold for semantic similarity
        })
    }

    /// Initialize database schema
    fn init_schema(pool: &Pool<SqliteConnectionManager>, dimension: usize) -> Result<(), String> {
        let conn = pool
            .get()
            .map_err(|e| format!("Failed to get connection: {}", e))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cache_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                response TEXT NOT NULL,
                embedding BLOB NOT NULL,
                timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                hit_count INTEGER NOT NULL DEFAULT 0,
                metadata TEXT,
                dimension INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| format!("Failed to create table: {}", e))?;

        // Create index on timestamp for cleanup
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON cache_entries(timestamp)",
            [],
        )
        .map_err(|e| format!("Failed to create index: {}", e))?;

        // Create index on hit_count for popularity-based eviction
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_hit_count ON cache_entries(hit_count)",
            [],
        )
        .map_err(|e| format!("Failed to create index: {}", e))?;

        // Store dimension in a metadata table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| format!("Failed to create metadata table: {}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES ('dimension', ?1)",
            [dimension.to_string()],
        )
        .map_err(|e| format!("Failed to store dimension: {}", e))?;

        Ok(())
    }

    /// Generate embedding for text
    fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        self.embedding_model.embed(text)
    }

    /// Serialize embedding to bytes for storage
    fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(embedding.len() * 4);
        for &val in embedding {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        bytes
    }

    /// Deserialize embedding from bytes
    fn bytes_to_embedding(bytes: &[u8]) -> Result<Vec<f32>, String> {
        if bytes.len() % 4 != 0 {
            return Err("Invalid embedding byte length".to_string());
        }

        let mut embedding = Vec::with_capacity(bytes.len() / 4);
        for chunk in bytes.chunks(4) {
            let val = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            embedding.push(val);
        }
        Ok(embedding)
    }

    /// Cache a query-response pair
    pub fn cache(
        &self,
        query: &str,
        response: &str,
        metadata: Option<String>,
    ) -> Result<i64, String> {
        // Generate embedding
        let embedding = self.embed(query)?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // Serialize embedding
        let embedding_bytes = Self::embedding_to_bytes(&embedding);

        // Get connection from pool
        let conn = self
            .db
            .get()
            .map_err(|e| format!("Failed to get connection: {}", e))?;

        // Insert into database
        conn.execute(
            "INSERT INTO cache_entries (query, response, embedding, timestamp, hit_count, metadata, dimension)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![query, response, embedding_bytes, timestamp, 0i64, metadata, self.dimension as i64],
        ).map_err(|e| format!("Failed to insert cache entry: {}", e))?;

        Ok(conn.last_insert_rowid())
    }

    /// Search for similar cached responses
    pub fn search(&self, query: &str, k: usize) -> Result<Vec<(CacheEntry, f32)>, String> {
        // Generate query embedding
        let query_embedding = self.embed(query)?;

        // Get connection from pool
        let conn = self
            .db
            .get()
            .map_err(|e| format!("Failed to get connection: {}", e))?;

        // Load all entries (in production, would use vector index for efficiency)
        let mut stmt = conn
            .prepare(
                "SELECT id, query, response, embedding, timestamp, hit_count, metadata
             FROM cache_entries
             ORDER BY hit_count DESC
             LIMIT ?1",
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let entries = stmt
            .query_map([k as i64 * 10], |row| {
                let embedding_bytes: Vec<u8> = row.get(3).unwrap_or_default();
                let embedding = Self::bytes_to_embedding(&embedding_bytes).unwrap_or_default();

                Ok(CacheEntry {
                    id: row.get(0).unwrap_or(0),
                    query: row.get(1).unwrap_or_default(),
                    response: row.get(2).unwrap_or_default(),
                    embedding,
                    timestamp: row.get(4).unwrap_or(0),
                    hit_count: row.get(5).unwrap_or(0),
                    metadata: row.get(6).ok(),
                })
            })
            .map_err(|e| format!("Failed to query cache: {}", e))?;

        // Compute similarity and filter
        let mut results = Vec::new();
        for entry_result in entries {
            let entry = entry_result.map_err(|e| format!("Failed to read entry: {}", e))?;
            let similarity = cosine_similarity(&query_embedding, &entry.embedding);

            if similarity >= self.similarity_threshold {
                results.push((entry, similarity));
            }
        }

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);

        Ok(results)
    }

    /// Get best matching cached response
    pub fn get(&self, query: &str) -> Result<Option<String>, String> {
        let results = self.search(query, 1)?;

        if let Some((entry, _similarity)) = results.first() {
            // Increment hit count
            let conn = self
                .db
                .get()
                .map_err(|e| format!("Failed to get connection: {}", e))?;
            conn.execute(
                "UPDATE cache_entries SET hit_count = hit_count + 1 WHERE id = ?1",
                [entry.id],
            )
            .map_err(|e| format!("Failed to update hit count: {}", e))?;

            Ok(Some(entry.response.clone()))
        } else {
            Ok(None)
        }
    }

    /// Remove old entries to stay within size limit
    pub fn evict_old(&self, max_entries: usize, max_age_days: u32) -> Result<usize, String> {
        let conn = self
            .db
            .get()
            .map_err(|e| format!("Failed to get connection: {}", e))?;

        let cutoff_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64 - (max_age_days as i64 * 86400))
            .unwrap_or(0);

        // First, get count of entries to delete
        let mut stmt = conn
            .prepare(
                "SELECT COUNT(*) FROM cache_entries 
             WHERE timestamp < ?1 
             AND hit_count = 0",
            )
            .map_err(|e| format!("Failed to count old entries: {}", e))?;

        let to_delete: i64 = stmt
            .query_row([cutoff_timestamp], |row| row.get(0))
            .map_err(|e| format!("Failed to query count: {}", e))?;

        if to_delete == 0 {
            // If no unused old entries, check total count
            drop(stmt);
            if self.count()? <= max_entries as u64 {
                return Ok(0);
            }
        }

        // Delete old, unused entries
        let deleted = conn
            .execute(
                "DELETE FROM cache_entries 
             WHERE timestamp < ?1 
             AND hit_count = 0",
                [cutoff_timestamp],
            )
            .map_err(|e| format!("Failed to delete old entries: {}", e))?;

        // If still over limit, delete least popular
        if self.count()? > max_entries as u64 {
            let extra_to_delete = self.count()? - max_entries as u64;
            let extra_deleted = conn
                .execute(
                    "DELETE FROM cache_entries 
                 WHERE id IN (
                     SELECT id FROM cache_entries 
                     ORDER BY hit_count ASC 
                     LIMIT ?1
                 )",
                    [extra_to_delete as i64],
                )
                .map_err(|e| format!("Failed to delete unpopular entries: {}", e))?;

            Ok(deleted + extra_deleted)
        } else {
            Ok(deleted)
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats, String> {
        let conn = self
            .db
            .get()
            .map_err(|e| format!("Failed to get connection: {}", e))?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM cache_entries", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count entries: {}", e))?;

        let total_hits: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(hit_count), 0) FROM cache_entries",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let avg_hits = if count > 0 {
            total_hits as f64 / count as f64
        } else {
            0.0
        };

        // Get size on disk - use database path from connection
        let db_path = conn.path().unwrap_or("memory");
        let db_size = std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);

        Ok(CacheStats {
            entry_count: count as usize,
            total_hits: total_hits as usize,
            avg_hits_per_entry: avg_hits,
            db_size_bytes: db_size,
            dimension: self.dimension,
        })
    }

    /// Get total entry count
    pub fn count(&self) -> Result<u64, String> {
        let conn = self
            .db
            .get()
            .map_err(|e| format!("Failed to get connection: {}", e))?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM cache_entries", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count entries: {}", e))?;

        Ok(count as u64)
    }

    /// Clear all cache entries
    pub fn clear(&self) -> Result<(), String> {
        let conn = self
            .db
            .get()
            .map_err(|e| format!("Failed to get connection: {}", e))?;
        conn.execute("DELETE FROM cache_entries", [])
            .map_err(|e| format!("Failed to clear cache: {}", e))?;
        Ok(())
    }

    /// Set similarity threshold
    pub fn set_threshold(&mut self, threshold: f32) {
        self.similarity_threshold = threshold.clamp(0.0, 1.0);
    }

    /// Get similarity threshold
    pub fn threshold(&self) -> f32 {
        self.similarity_threshold
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_hits: usize,
    pub avg_hits_per_entry: f64,
    pub db_size_bytes: u64,
    pub dimension: usize,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cache Statistics:")?;
        writeln!(f, "  Entries: {}", self.entry_count)?;
        writeln!(f, "  Total hits: {}", self.total_hits)?;
        writeln!(f, "  Avg hits/entry: {:.2}", self.avg_hits_per_entry)?;
        writeln!(
            f,
            "  DB size: {:.2} MB",
            self.db_size_bytes as f64 / 1_048_576.0
        )?;
        writeln!(f, "  Embedding dim: {}", self.dimension)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_cache_basic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache.db");
        let model = Arc::new(EmbeddingModel::dummy());

        let mut cache = SemanticCache::new(&db_path, model).unwrap();

        // Cache a response
        cache
            .cache(
                "What is Rust?",
                "Rust is a systems programming language",
                None,
            )
            .unwrap();

        // Search for it
        let results = cache.search("Tell me about Rust", 5).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].1 > 0.85); // High similarity

        // Get best match
        let response = cache.get("What is Rust?").unwrap();
        assert!(response.is_some());
    }

    #[test]
    fn test_cache_eviction() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache.db");
        let model = Arc::new(EmbeddingModel::dummy());

        let mut cache = SemanticCache::new(&db_path, model).unwrap();

        // Add multiple entries
        for i in 0..10 {
            cache
                .cache(&format!("Query {}", i), &format!("Response {}", i), None)
                .unwrap();
        }

        assert_eq!(cache.count().unwrap(), 10);

        // Evict old entries (all are old since we didn't sleep)
        let deleted = cache.evict_old(5, 0).unwrap();
        assert!(deleted > 0);

        let stats = cache.stats().unwrap();
        assert!(stats.entry_count <= 5);
    }

    #[test]
    fn test_embedding_serialization() {
        let embedding = vec![1.0f32, 2.0, 3.0, 4.0];
        let bytes = SemanticCache::embedding_to_bytes(&embedding);
        let restored = SemanticCache::bytes_to_embedding(&bytes).unwrap();

        assert_eq!(embedding, restored);
    }
}

// Benchmark comment