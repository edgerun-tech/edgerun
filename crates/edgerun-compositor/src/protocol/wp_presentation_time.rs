//! wp_presentation — precise presentation timing protocol.
//!
//! Provides precise presentation timing information to clients.
//! Browsers use this for smooth video playback and animations.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

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
    // sig: uint, uint, uint, uint, uint, uint, uint

    /// discarded — the buffer was not displayed.
    pub const DISCARDED: u16 = 1;
    // sig: uint (reason)
}

/// Build a presented event.
///
/// `tv_sec` is split into hi/lo 32-bit parts to handle 64-bit values.
/// `tv_nsec` is nanoseconds within the second.
/// `refresh` is the refresh interval in nanoseconds (0 if unknown).
/// `seq` is the media stream sequence number, split into hi/lo.
/// `kind` is a bitfield of presentation kind flags.
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
///
/// `reason` indicates why the buffer was discarded:
/// - 0: presented (should use PRESENTED instead)
/// - 1: discarded (frame was replaced before presentation)
/// - 2: cancelled (surface was destroyed)
pub fn feedback_discarded_event(feedback_id: u32, reason: u32) -> Message {
    use presentation_feedback_event::DISCARDED;
    message_uint(feedback_id, DISCARDED, reason)
}
