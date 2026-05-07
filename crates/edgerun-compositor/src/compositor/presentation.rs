//! Presentation feedback runtime state.

use std::collections::HashMap;

/// Tracks pending presentation feedback objects per surface.
///
/// When a surface commits a new buffer, the previous feedback is superseded.
/// When a page flip completes, feedback objects are sent `presented` with
/// real VBLANK timing.
#[derive(Debug, Default)]
pub struct PresentationFeedbackTracker {
    /// surface_id -> Vec<(feedback_id, client_id)> for the committed buffer.
    pending: HashMap<u32, Vec<(u32, u32)>>,
    /// Frame sequence counter (64-bit, split into hi/lo for events).
    seq: u64,
}

impl PresentationFeedbackTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new feedback object for a surface.
    pub fn register(&mut self, surface_id: u32, feedback_id: u32, client_id: u32) {
        self.pending
            .entry(surface_id)
            .or_default()
            .push((feedback_id, client_id));
    }

    /// Supersede all pending feedback for a surface.
    pub fn supersede(&mut self, surface_id: u32) -> Vec<(u32, u32)> {
        self.pending.remove(&surface_id).unwrap_or_default()
    }

    /// Drain feedback objects for a completed presentation.
    pub fn take_presented(&mut self) -> Vec<(u32, u32)> {
        let result = self.pending.drain().flat_map(|(_, v)| v).collect();
        self.seq += 1;
        result
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }

    pub fn seq_lo(&self) -> u32 {
        self.seq as u32
    }

    pub fn seq_hi(&self) -> u32 {
        (self.seq >> 32) as u32
    }

    pub fn clock_timestamp(&self) -> (u32, u32, u32) {
        let _ = self;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let sec = now.as_secs();
        ((sec >> 32) as u32, sec as u32, now.subsec_nanos())
    }
}
