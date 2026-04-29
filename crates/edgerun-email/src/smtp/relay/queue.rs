//! Persistent outbound mail queue.
//!
//! Uses in-memory HashMaps guarded by async RwLock, persisted to disk
//! via `spawn_blocking` for crash safety.

use crate::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};

use super::now_secs;
use crate::rt::{spawn_blocking, RwLock};
use edgerun_encoding::byteorder::{read_i32_le, read_i64_le};
use edgerun_encoding::string_field::{
    decode_bytes_u64, decode_string_field_u64, encode_bytes_u64, encode_string_field_u64,
    StringFieldError,
};

// ===========================================================================
// Record Types
// ===========================================================================

/// A queued outbound message.
#[derive(Debug, Clone)]
pub struct MailMessageRecord {
    pub message_id: String,
    pub envelope_sender: String,
    pub recipients: Vec<String>,
    pub data: Vec<u8>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub next_retry_time: i64,
    pub status: MailStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Per-recipient delivery status.
#[derive(Debug, Clone)]
pub struct RecipientStatus {
    pub message_id: String,
    pub recipient: String,
    pub status: RecipientStatusType,
    pub last_attempt: Option<i64>,
    pub failure_reason: Option<String>,
    pub delivered_at: Option<i64>,
}

/// Status of a single recipient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecipientStatusType {
    Pending,
    Sent,
    Delivered,
    Failed,
    Bounced,
}

/// Retry queue entry — messages due for delivery.
#[derive(Debug, Clone)]
pub struct RetryEntry {
    pub message_id: String,
    pub next_retry_time: i64,
    pub retry_count: i32,
}

/// Audit trail entry for delivery attempts.
#[derive(Debug, Clone)]
pub struct SendLogEntry {
    pub message_id: String,
    pub recipient: String,
    pub attempt: i32,
    pub timestamp: i64,
    pub success: bool,
    pub error: Option<String>,
    pub remote_mta: Option<String>,
}

/// Message status (lifecycle state).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailStatus {
    Queued,
    Sending,
    Delivered,
    Retrying,
    Bounced,
}

impl MailStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MailStatus::Queued => "queued",
            MailStatus::Sending => "sending",
            MailStatus::Delivered => "delivered",
            MailStatus::Retrying => "retrying",
            MailStatus::Bounced => "bounced",
        }
    }
}

impl RecipientStatusType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecipientStatusType::Pending => "pending",
            RecipientStatusType::Sent => "sent",
            RecipientStatusType::Delivered => "delivered",
            RecipientStatusType::Failed => "failed",
            RecipientStatusType::Bounced => "bounced",
        }
    }
}

// ===========================================================================
// Binary serialization helpers
// ===========================================================================

fn write_str(w: &mut Vec<u8>, s: &str) {
    encode_string_field_u64(s, w).expect("u64 string length prefix cannot overflow on this target");
}

fn read_str(r: &mut Cursor<&[u8]>) -> io::Result<String> {
    let mut cursor = r.position() as usize;
    let value =
        decode_string_field_u64(r.get_ref(), &mut cursor).map_err(map_string_field_error)?;
    r.set_position(cursor as u64);
    Ok(value)
}

fn write_vec_u8(w: &mut Vec<u8>, v: &[u8]) {
    encode_bytes_u64(v, w).expect("u64 byte length prefix cannot overflow on this target");
}

fn read_vec_u8(r: &mut Cursor<&[u8]>) -> io::Result<Vec<u8>> {
    let mut cursor = r.position() as usize;
    let value = decode_bytes_u64(r.get_ref(), &mut cursor).map_err(map_string_field_error)?;
    r.set_position(cursor as u64);
    Ok(value)
}

fn map_string_field_error(error: StringFieldError) -> io::Error {
    let kind = match error {
        StringFieldError::InvalidUtf8 => io::ErrorKind::InvalidData,
        StringFieldError::TruncatedInput | StringFieldError::LengthExceedsInput => {
            io::ErrorKind::UnexpectedEof
        }
    };
    io::Error::new(kind, error)
}

fn write_i64(w: &mut Vec<u8>, v: i64) {
    w.extend_from_slice(&v.to_le_bytes());
}

fn read_i64(r: &mut Cursor<&[u8]>) -> io::Result<i64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(read_i64_le(&buf, 0))
}

fn write_i32(w: &mut Vec<u8>, v: i32) {
    w.extend_from_slice(&v.to_le_bytes());
}

fn read_i32(r: &mut Cursor<&[u8]>) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(read_i32_le(&buf, 0))
}

fn write_option_str(w: &mut Vec<u8>, s: &Option<String>) {
    match s {
        Some(v) => {
            w.push(1);
            write_str(w, v);
        }
        None => w.push(0),
    }
}

fn read_option_str(r: &mut Cursor<&[u8]>) -> io::Result<Option<String>> {
    let mut flag = [0u8; 1];
    r.read_exact(&mut flag)?;
    if flag[0] == 1 {
        Ok(Some(read_str(r)?))
    } else {
        Ok(None)
    }
}

// ===========================================================================
// MailIndex
// ===========================================================================

pub struct MailIndex {
    messages: RwLock<HashMap<String, MailMessageRecord>>,
    recipients: RwLock<HashMap<(String, String), RecipientStatus>>,
    retry_queue: RwLock<Vec<RetryEntry>>,
    send_log: RwLock<Vec<SendLogEntry>>,
    data_root: PathBuf,
    next_message_id: AtomicI64,
}

impl MailIndex {
    pub async fn open(data_root: &Path) -> io::Result<Self> {
        let idx_dir = data_root.join("mail");
        spawn_blocking(move || fs::create_dir_all(&idx_dir))
            .await
            .map_err(io::Error::other)??;

        let index = Self {
            messages: RwLock::new(HashMap::new()),
            recipients: RwLock::new(HashMap::new()),
            retry_queue: RwLock::new(Vec::new()),
            send_log: RwLock::new(Vec::new()),
            data_root: data_root.to_path_buf(),
            next_message_id: AtomicI64::new(0),
        };

        index.load().await?;
        Ok(index)
    }

    fn idx_dir(&self) -> PathBuf {
        self.data_root.join("mail")
    }

    async fn load(&self) -> io::Result<()> {
        let dir = self.idx_dir();

        // Load messages.bin
        let data = spawn_blocking({
            let path = dir.join("messages.bin");
            move || fs::read(&path)
        })
        .await
        .map_err(io::Error::other)?;
        if let Ok(data) = data {
            let mut cursor = Cursor::new(data.as_slice());
            while cursor.position() < data.len() as u64 {
                let mut msg = read_message_record(&mut cursor)?;
                let recover_sending = matches!(msg.status, MailStatus::Sending);
                if recover_sending {
                    msg.status = MailStatus::Retrying;
                    msg.next_retry_time = 0;
                }
                self.next_message_id.fetch_max(
                    msg.message_id.parse::<i64>().unwrap_or(0),
                    Ordering::Relaxed,
                );
                if recover_sending {
                    self.retry_queue.write().await.push(RetryEntry {
                        message_id: msg.message_id.clone(),
                        next_retry_time: msg.next_retry_time,
                        retry_count: msg.retry_count,
                    });
                }
                self.messages
                    .write()
                    .await
                    .insert(msg.message_id.clone(), msg);
            }
        }

        // Load recipients.bin
        let data = spawn_blocking({
            let path = dir.join("recipients.bin");
            move || fs::read(&path)
        })
        .await
        .map_err(io::Error::other)?;
        if let Ok(data) = data {
            let mut cursor = Cursor::new(data.as_slice());
            while cursor.position() < data.len() as u64 {
                let rec = read_recipient_status(&mut cursor)?;
                let key = (rec.message_id.clone(), rec.recipient.clone());
                self.recipients.write().await.insert(key, rec);
            }
        }

        // Load retry_queue.bin
        let data = spawn_blocking({
            let path = dir.join("retry_queue.bin");
            move || fs::read(&path)
        })
        .await
        .map_err(io::Error::other)?;
        if let Ok(data) = data {
            let mut cursor = Cursor::new(data.as_slice());
            while cursor.position() < data.len() as u64 {
                let entry = read_retry_entry(&mut cursor)?;
                self.retry_queue.write().await.push(entry);
            }
        }

        // Load send_log.bin
        let data = spawn_blocking({
            let path = dir.join("send_log.bin");
            move || fs::read(&path)
        })
        .await
        .map_err(io::Error::other)?;
        if let Ok(data) = data {
            let mut cursor = Cursor::new(data.as_slice());
            while cursor.position() < data.len() as u64 {
                let entry = read_send_log_entry(&mut cursor)?;
                self.send_log.write().await.push(entry);
            }
        }

        Ok(())
    }

    pub async fn save(&self) -> io::Result<()> {
        let dir = self.idx_dir();

        // Clone data out of the RwLock guards before entering spawn_blocking.
        let messages: Vec<MailMessageRecord> = {
            let guard = self.messages.read().await;
            guard.values().cloned().collect()
        };
        let recipients: Vec<RecipientStatus> = {
            let guard = self.recipients.read().await;
            guard.values().cloned().collect()
        };
        let retry_queue: Vec<RetryEntry> = {
            let guard = self.retry_queue.read().await;
            guard.iter().cloned().collect()
        };
        let send_log: Vec<SendLogEntry> = {
            let guard = self.send_log.read().await;
            guard.iter().cloned().collect()
        };

        spawn_blocking(move || {
            fs::create_dir_all(&dir)?;

            let mut buf = Vec::new();
            for msg in &messages {
                write_message_record(&mut buf, msg);
            }
            fs::write(dir.join("messages.bin"), buf)?;

            let mut buf = Vec::new();
            for rec in &recipients {
                write_recipient_status(&mut buf, rec);
            }
            fs::write(dir.join("recipients.bin"), buf)?;

            let mut buf = Vec::new();
            for entry in &retry_queue {
                write_retry_entry(&mut buf, entry);
            }
            fs::write(dir.join("retry_queue.bin"), buf)?;

            let mut buf = Vec::new();
            for entry in &send_log {
                write_send_log_entry(&mut buf, entry);
            }
            fs::write(dir.join("send_log.bin"), buf)?;

            Ok::<_, io::Error>(())
        })
        .await
        .map_err(io::Error::other)?
    }

    pub async fn enqueue_message(
        &self,
        message_id: &str,
        envelope_sender: &str,
        recipients: Vec<String>,
        data: Vec<u8>,
        max_retries: i32,
    ) -> io::Result<()> {
        let now = now_secs();
        let record = MailMessageRecord {
            message_id: message_id.to_string(),
            envelope_sender: envelope_sender.to_string(),
            recipients: recipients.clone(),
            data,
            retry_count: 0,
            max_retries,
            next_retry_time: 0,
            status: MailStatus::Queued,
            created_at: now,
            updated_at: now,
        };

        {
            let mut recs = self.recipients.write().await;
            for recipient in &recipients {
                recs.insert(
                    (message_id.to_string(), recipient.clone()),
                    RecipientStatus {
                        message_id: message_id.to_string(),
                        recipient: recipient.clone(),
                        status: RecipientStatusType::Pending,
                        last_attempt: None,
                        failure_reason: None,
                        delivered_at: None,
                    },
                );
            }
        }

        self.retry_queue.write().await.push(RetryEntry {
            message_id: message_id.to_string(),
            next_retry_time: 0,
            retry_count: 0,
        });

        self.messages
            .write()
            .await
            .insert(message_id.to_string(), record);
        self.save().await
    }

    /// Get messages due for delivery.
    pub async fn dequeue_due(&self, now: i64, max_batch: usize) -> Vec<MailMessageRecord> {
        // Read both maps — guards are dropped immediately after.
        let messages: Vec<(String, MailMessageRecord)> = {
            let m = self.messages.read().await;
            m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        let retry_entries: Vec<RetryEntry> = {
            let rq = self.retry_queue.read().await;
            rq.iter().cloned().collect()
        };

        // Determine which entries are due
        let mut due_ids = Vec::new();
        for entry in &retry_entries {
            if entry.next_retry_time <= now || entry.next_retry_time == 0 {
                if let Some((_id, msg)) = messages.iter().find(|(id, _)| id == &entry.message_id) {
                    if matches!(msg.status, MailStatus::Queued | MailStatus::Retrying) {
                        due_ids.push(entry.message_id.clone());
                        if due_ids.len() >= max_batch {
                            break;
                        }
                    }
                }
            }
        }

        // Collect the actual messages, updating their status
        let mut due = Vec::new();
        {
            let mut msgs = self.messages.write().await;
            for id in &due_ids {
                if let Some(msg) = msgs.get_mut(id) {
                    msg.status = MailStatus::Sending;
                    msg.updated_at = now_secs();
                    due.push(msg.clone());
                }
            }
        }

        // Remove processed entries from retry queue
        {
            let mut rq = self.retry_queue.write().await;
            rq.retain(|e| !due_ids.contains(&e.message_id));
        }

        due
    }

    pub async fn mark_sending(&self, message_id: &str) -> io::Result<()> {
        if let Some(msg) = self.messages.write().await.get_mut(message_id) {
            msg.status = MailStatus::Sending;
            msg.updated_at = now_secs();
        }
        self.save().await
    }

    pub async fn mark_delivered(
        &self,
        message_id: &str,
        recipient: &str,
        remote_mta: Option<&str>,
    ) -> io::Result<()> {
        let now = now_secs();
        {
            let mut recs = self.recipients.write().await;
            if let Some(rec) = recs.get_mut(&(message_id.to_string(), recipient.to_string())) {
                rec.status = RecipientStatusType::Delivered;
                rec.delivered_at = Some(now);
            }
        }

        self.send_log.write().await.push(SendLogEntry {
            message_id: message_id.to_string(),
            recipient: recipient.to_string(),
            attempt: 1,
            timestamp: now,
            success: true,
            error: None,
            remote_mta: remote_mta.map(String::from),
        });

        let all_delivered = {
            let recs = self.recipients.read().await;
            recs.iter()
                .filter(|((mid, _), _)| mid == message_id)
                .all(|(_, rec)| matches!(rec.status, RecipientStatusType::Delivered))
        };

        if all_delivered {
            if let Some(msg) = self.messages.write().await.get_mut(message_id) {
                msg.status = MailStatus::Delivered;
                msg.updated_at = now;
            }
        }

        self.save().await
    }

    pub async fn mark_failed(
        &self,
        message_id: &str,
        recipient: &str,
        error: &str,
        remote_mta: Option<&str>,
    ) -> io::Result<()> {
        let now = now_secs();
        {
            let mut recs = self.recipients.write().await;
            if let Some(rec) = recs.get_mut(&(message_id.to_string(), recipient.to_string())) {
                rec.status = RecipientStatusType::Failed;
                rec.last_attempt = Some(now);
                rec.failure_reason = Some(error.to_string());
            }
        }

        self.send_log.write().await.push(SendLogEntry {
            message_id: message_id.to_string(),
            recipient: recipient.to_string(),
            attempt: 1,
            timestamp: now,
            success: false,
            error: Some(error.to_string()),
            remote_mta: remote_mta.map(String::from),
        });

        self.save().await
    }

    pub async fn schedule_retry(
        &self,
        message_id: &str,
        retry_count: i32,
        next_retry_time: i64,
    ) -> io::Result<()> {
        let now = now_secs();
        {
            let mut msgs = self.messages.write().await;
            if let Some(msg) = msgs.get_mut(message_id) {
                msg.retry_count = retry_count;
                msg.next_retry_time = next_retry_time;
                msg.status = MailStatus::Retrying;
                msg.updated_at = now;
            }
        }

        self.retry_queue.write().await.push(RetryEntry {
            message_id: message_id.to_string(),
            next_retry_time,
            retry_count,
        });

        self.save().await
    }

    pub async fn mark_bounced(&self, message_id: &str) -> io::Result<()> {
        let now = now_secs();
        if let Some(msg) = self.messages.write().await.get_mut(message_id) {
            msg.status = MailStatus::Bounced;
            msg.updated_at = now;
        }
        self.save().await
    }

    pub async fn get_message(&self, message_id: &str) -> Option<MailMessageRecord> {
        self.messages.read().await.get(message_id).cloned()
    }

    /// Check if a specific recipient for a message has been delivered.
    pub async fn is_recipient_delivered(&self, message_id: &str, recipient: &str) -> bool {
        let recs = self.recipients.read().await;
        recs.get(&(message_id.to_string(), recipient.to_string()))
            .map(|r| matches!(r.status, RecipientStatusType::Delivered))
            .unwrap_or(false)
    }

    pub async fn get_failed_recipients(&self, message_id: &str) -> Vec<RecipientStatus> {
        self.recipients
            .read()
            .await
            .iter()
            .filter(|((mid, _), rec)| {
                mid == message_id
                    && matches!(
                        rec.status,
                        RecipientStatusType::Failed | RecipientStatusType::Bounced
                    )
            })
            .map(|(_, rec)| rec.clone())
            .collect()
    }

    pub async fn list_queued(&self) -> Vec<MailMessageRecord> {
        self.messages
            .read()
            .await
            .values()
            .filter(|m| matches!(m.status, MailStatus::Queued | MailStatus::Retrying))
            .cloned()
            .collect()
    }
}

// ===========================================================================
// Serialization
// ===========================================================================

fn write_message_record(w: &mut Vec<u8>, msg: &MailMessageRecord) {
    write_str(w, &msg.message_id);
    write_str(w, &msg.envelope_sender);
    write_str(w, &msg.recipients.join("\x00"));
    write_vec_u8(w, &msg.data);
    write_i32(w, msg.retry_count);
    write_i32(w, msg.max_retries);
    write_i64(w, msg.next_retry_time);
    write_str(w, msg.status.as_str());
    write_i64(w, msg.created_at);
    write_i64(w, msg.updated_at);
}

fn read_message_record(r: &mut Cursor<&[u8]>) -> io::Result<MailMessageRecord> {
    let message_id = read_str(r)?;
    let envelope_sender = read_str(r)?;
    let recipients_str = read_str(r)?;
    let data = read_vec_u8(r)?;
    let retry_count = read_i32(r)?;
    let max_retries = read_i32(r)?;
    let next_retry_time = read_i64(r)?;
    let status_str = read_str(r)?;
    let created_at = read_i64(r)?;
    let updated_at = read_i64(r)?;

    let status = match status_str.as_str() {
        "queued" => MailStatus::Queued,
        "sending" => MailStatus::Sending,
        "delivered" => MailStatus::Delivered,
        "retrying" => MailStatus::Retrying,
        "bounced" => MailStatus::Bounced,
        _ => MailStatus::Queued,
    };

    Ok(MailMessageRecord {
        message_id,
        envelope_sender,
        recipients: if recipients_str.is_empty() {
            Vec::new()
        } else {
            recipients_str.split('\x00').map(String::from).collect()
        },
        data,
        retry_count,
        max_retries,
        next_retry_time,
        status,
        created_at,
        updated_at,
    })
}

fn write_recipient_status(w: &mut Vec<u8>, rec: &RecipientStatus) {
    write_str(w, &rec.message_id);
    write_str(w, &rec.recipient);
    write_str(w, rec.status.as_str());
    write_option_str(w, &rec.last_attempt.map(|t| t.to_string()));
    write_option_str(w, &rec.failure_reason);
    write_option_str(w, &rec.delivered_at.map(|t| t.to_string()));
}

fn read_recipient_status(r: &mut Cursor<&[u8]>) -> io::Result<RecipientStatus> {
    let message_id = read_str(r)?;
    let recipient = read_str(r)?;
    let status_str = read_str(r)?;
    let last_attempt_str = read_option_str(r)?;
    let failure_reason = read_option_str(r)?;
    let delivered_at_str = read_option_str(r)?;

    let status = match status_str.as_str() {
        "pending" => RecipientStatusType::Pending,
        "sent" => RecipientStatusType::Sent,
        "delivered" => RecipientStatusType::Delivered,
        "failed" => RecipientStatusType::Failed,
        "bounced" => RecipientStatusType::Bounced,
        _ => RecipientStatusType::Pending,
    };

    let last_attempt = last_attempt_str.and_then(|s| s.parse::<i64>().ok());
    let delivered_at = delivered_at_str.and_then(|s| s.parse::<i64>().ok());

    Ok(RecipientStatus {
        message_id,
        recipient,
        status,
        last_attempt,
        failure_reason,
        delivered_at,
    })
}

fn write_retry_entry(w: &mut Vec<u8>, entry: &RetryEntry) {
    write_str(w, &entry.message_id);
    write_i64(w, entry.next_retry_time);
    write_i32(w, entry.retry_count);
}

fn read_retry_entry(r: &mut Cursor<&[u8]>) -> io::Result<RetryEntry> {
    let message_id = read_str(r)?;
    let next_retry_time = read_i64(r)?;
    let retry_count = read_i32(r)?;
    Ok(RetryEntry {
        message_id,
        next_retry_time,
        retry_count,
    })
}

fn write_send_log_entry(w: &mut Vec<u8>, entry: &SendLogEntry) {
    write_str(w, &entry.message_id);
    write_str(w, &entry.recipient);
    write_i32(w, entry.attempt);
    write_i64(w, entry.timestamp);
    w.push(if entry.success { 1 } else { 0 });
    write_option_str(w, &entry.error);
    write_option_str(w, &entry.remote_mta);
}

fn read_send_log_entry(r: &mut Cursor<&[u8]>) -> io::Result<SendLogEntry> {
    let message_id = read_str(r)?;
    let recipient = read_str(r)?;
    let attempt = read_i32(r)?;
    let timestamp = read_i64(r)?;
    let mut success_flag = [0u8; 1];
    r.read_exact(&mut success_flag)?;
    let success = success_flag[0] == 1;
    let error = read_option_str(r)?;
    let remote_mta = read_option_str(r)?;
    Ok(SendLogEntry {
        message_id,
        recipient,
        attempt,
        timestamp,
        success,
        error,
        remote_mta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_record_serialization() {
        let msg = MailMessageRecord {
            message_id: "test1".to_string(),
            envelope_sender: "sender@example.com".to_string(),
            recipients: vec!["a@dest.com".to_string(), "b@dest.com".to_string()],
            data: b"hello body".to_vec(),
            retry_count: 2,
            max_retries: 5,
            next_retry_time: 1700000000,
            status: MailStatus::Retrying,
            created_at: 1699999000,
            updated_at: 1700000000,
        };

        let mut buf = Vec::new();
        write_message_record(&mut buf, &msg);

        let mut cursor = Cursor::new(buf.as_slice());
        let decoded = read_message_record(&mut cursor).unwrap();

        assert_eq!(decoded.message_id, msg.message_id);
        assert_eq!(decoded.envelope_sender, msg.envelope_sender);
        assert_eq!(decoded.recipients, msg.recipients);
        assert_eq!(decoded.data, msg.data);
        assert_eq!(decoded.retry_count, msg.retry_count);
        assert_eq!(decoded.max_retries, msg.max_retries);
        assert_eq!(decoded.next_retry_time, msg.next_retry_time);
        assert_eq!(decoded.status, msg.status);
        assert_eq!(decoded.created_at, msg.created_at);
    }

    #[test]
    fn test_recipient_status_serialization() {
        let rec = RecipientStatus {
            message_id: "msg1".to_string(),
            recipient: "user@dest.com".to_string(),
            status: RecipientStatusType::Failed,
            last_attempt: Some(1700000000),
            failure_reason: Some("connection refused".to_string()),
            delivered_at: None,
        };

        let mut buf = Vec::new();
        write_recipient_status(&mut buf, &rec);

        let mut cursor = Cursor::new(buf.as_slice());
        let decoded = read_recipient_status(&mut cursor).unwrap();

        assert_eq!(decoded.message_id, rec.message_id);
        assert_eq!(decoded.recipient, rec.recipient);
        assert_eq!(decoded.status, rec.status);
        assert_eq!(decoded.last_attempt, rec.last_attempt);
        assert_eq!(decoded.failure_reason, rec.failure_reason);
        assert_eq!(decoded.delivered_at, rec.delivered_at);
    }

    #[test]
    fn test_retry_entry_serialization() {
        let entry = RetryEntry {
            message_id: "msg2".to_string(),
            next_retry_time: 1700000600,
            retry_count: 3,
        };

        let mut buf = Vec::new();
        write_retry_entry(&mut buf, &entry);

        let mut cursor = Cursor::new(buf.as_slice());
        let decoded = read_retry_entry(&mut cursor).unwrap();

        assert_eq!(decoded.message_id, entry.message_id);
        assert_eq!(decoded.next_retry_time, entry.next_retry_time);
        assert_eq!(decoded.retry_count, entry.retry_count);
    }

    #[test]
    fn test_send_log_entry_serialization() {
        let entry = SendLogEntry {
            message_id: "msg3".to_string(),
            recipient: "user@dest.com".to_string(),
            attempt: 2,
            timestamp: 1700000000,
            success: false,
            error: Some("timeout".to_string()),
            remote_mta: Some("mx.dest.com".to_string()),
        };

        let mut buf = Vec::new();
        write_send_log_entry(&mut buf, &entry);

        let mut cursor = Cursor::new(buf.as_slice());
        let decoded = read_send_log_entry(&mut cursor).unwrap();

        assert_eq!(decoded.message_id, entry.message_id);
        assert_eq!(decoded.recipient, entry.recipient);
        assert_eq!(decoded.attempt, entry.attempt);
        assert_eq!(decoded.timestamp, entry.timestamp);
        assert_eq!(decoded.success, entry.success);
        assert_eq!(decoded.error, entry.error);
        assert_eq!(decoded.remote_mta, entry.remote_mta);
    }

    #[test]
    fn test_status_string_roundtrip() {
        let statuses = [
            MailStatus::Queued,
            MailStatus::Sending,
            MailStatus::Delivered,
            MailStatus::Retrying,
            MailStatus::Bounced,
        ];
        for s in &statuses {
            let msg = MailMessageRecord {
                message_id: "x".to_string(),
                envelope_sender: "s@e.com".to_string(),
                recipients: vec![],
                data: vec![],
                retry_count: 0,
                max_retries: 0,
                next_retry_time: 0,
                status: s.clone(),
                created_at: 0,
                updated_at: 0,
            };
            let mut buf = Vec::new();
            write_message_record(&mut buf, &msg);
            let mut cursor = Cursor::new(buf.as_slice());
            let decoded = read_message_record(&mut cursor).unwrap();
            assert_eq!(decoded.status, msg.status);
        }
    }
}
