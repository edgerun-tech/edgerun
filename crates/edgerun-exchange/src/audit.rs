//! Audit logger for provider calls.
//!
//! Logs internal audit events (provider code, operation, correlation id,
//! request hash, response hash, status, latency, redacted error).

use alloc::string::String;
use core::fmt;

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub provider_code: String, // internal only
    pub operation: String,
    pub correlation_id: String,
    pub request_hash: String,
    pub response_hash: String,
    pub status_code: u16,
    pub latency_ms: u64,
    pub redacted_error: Option<String>,
    pub timestamp_ms: u64,
}

/// Audit logger trait.
pub trait AuditLogger {
    fn log(&self, entry: AuditEntry);
}

/// Simple in-memory audit logger (for now).
#[derive(Debug, Default)]
pub struct SimpleAuditLogger {
    entries: alloc::vec::Vec<AuditEntry>,
}

impl AuditLogger for SimpleAuditLogger {
    fn log(&self, entry: AuditEntry) {
        // TODO: store or forward to persistent storage
        edgerun_log::info!(
            "AUDIT: provider={} op={} status={} latency={}ms",
            entry.provider_code,
            entry.operation,
            entry.status_code,
            entry.latency_ms
        );
    }
}
