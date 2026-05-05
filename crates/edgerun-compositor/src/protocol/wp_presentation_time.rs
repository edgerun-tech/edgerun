//! wp_presentation — precise presentation timing protocol.
//!
//! Provides precise presentation timing information to clients.
//! Browsers use this for smooth video playback and animations.

use crate::wire::encode::*;
use crate::wire::{ArgType, Message};

pub const WP_PRESENTATION: &str = "wp_presentation";
pub const WP_PRESENTATION_VERSION: u32 = 1;

pub mod presentation_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const FEEDBACK: u16 = 1;
    pub const FEEDBACK_SIG: &[ArgType] = &[ArgType::Object, ArgType::NewId]; // surface, wp_presentation_feedback
}

// ─── wp_presentation_feedback ────────────────────────────────

pub const WP_PRESENTATION_FEEDBACK: &str = "wp_presentation_feedback";
pub const WP_PRESENTATION_FEEDBACK_VERSION: u32 = 1;

pub mod presentation_feedback_event {
    /// presented — the buffer was displayed.
    /// Args: tv_sec_hi, tv_sec_lo, tv_nsec, refresh, seq_hi, seq_lo, kind
    pub const PRESENTED: u16 = 0;

    /// discarded — the buffer was not displayed.
    pub const DISCARDED: u16 = 1;
}

/// Presentation kind flags (bitfield).
pub mod presentation_kind {
    pub const VSYNC: u32 = 0x01;
    pub const HW_CLOCK: u32 = 0x02;
    pub const HW_COMPLETION: u32 = 0x04;
}

/// Discarded reasons.
pub mod discard_reason {
    pub const DESTROYED: u32 = 0;
    pub const SUPERSEDED: u32 = 1;
}

/// Build a presented event with 64-bit timestamp split into hi/lo.
pub fn feedback_presented_event(
    feedback_id: u32,
    tv_sec_hi: u32,
    tv_sec_lo: u32,
    tv_nsec: u32,
    refresh: u32,
    seq_hi: u32,
    seq_lo: u32,
    kind: u32,
) -> Message {
    use presentation_feedback_event::PRESENTED;
    let mut args = Vec::new();
    args.extend_from_slice(&tv_sec_hi.to_le_bytes());
    args.extend_from_slice(&tv_sec_lo.to_le_bytes());
    args.extend_from_slice(&tv_nsec.to_le_bytes());
    args.extend_from_slice(&refresh.to_le_bytes());
    args.extend_from_slice(&seq_hi.to_le_bytes());
    args.extend_from_slice(&seq_lo.to_le_bytes());
    args.extend_from_slice(&kind.to_le_bytes());
    Message {
        sender_id: feedback_id,
        opcode: PRESENTED,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a discarded event.
pub fn feedback_discarded_event(feedback_id: u32, reason: u32) -> Message {
    use presentation_feedback_event::DISCARDED;
    message_uint(feedback_id, DISCARDED, reason)
}

/// Tracks pending presentation feedback objects per surface.
///
/// When a surface commits a new buffer, the previous feedback is superseded.
/// When a page flip completes, feedback objects are sent `presented` with
/// real VBLANK timing.
#[derive(Debug, Default)]
pub struct PresentationFeedbackTracker {
    /// surface_id → Vec<(feedback_id, client_id)> for the currently committed buffer.
    pending: std::collections::HashMap<u32, Vec<(u32, u32)>>,
    /// Frame sequence counter (64-bit, split into hi/lo for events).
    seq: u64,
}

impl PresentationFeedbackTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new feedback object for a surface.
    /// Multiple feedbacks can be registered per surface.
    pub fn register(&mut self, surface_id: u32, feedback_id: u32, client_id: u32) {
        self.pending
            .entry(surface_id)
            .or_default()
            .push((feedback_id, client_id));
    }

    /// Called when a surface commits a new buffer before the previous
    /// frame was presented. Supersedes all pending feedback for that surface.
    /// Returns the list of (feedback_id, client_id) that should receive `discarded`.
    pub fn supersede(&mut self, surface_id: u32) -> Vec<(u32, u32)> {
        self.pending.remove(&surface_id).unwrap_or_default()
    }

    /// Called when a page flip completes successfully.
    /// Returns all pending feedback objects that should receive `presented`,
    /// and increments the frame sequence counter.
    pub fn take_presented(&mut self) -> Vec<(u32, u32)> {
        let result: Vec<_> = self.pending.drain().flat_map(|(_, v)| v).collect();
        self.seq += 1;
        result
    }

    /// Clear all pending feedbacks (e.g., on compositor shutdown).
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Current frame sequence number (low 32 bits).
    pub fn seq_lo(&self) -> u32 {
        self.seq as u32
    }

    /// Current frame sequence number (high 32 bits).
    pub fn seq_hi(&self) -> u32 {
        (self.seq >> 32) as u32
    }

    /// Get current monotonic timestamp as (sec_hi, sec_lo, nsec).
    pub fn clock_timestamp(&self) -> (u32, u32, u32) {
        let _ = self; // static method behavior
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let sec = now.as_secs();
        let nsec = now.subsec_nanos();
        ((sec >> 32) as u32, sec as u32, nsec)
    }
}
