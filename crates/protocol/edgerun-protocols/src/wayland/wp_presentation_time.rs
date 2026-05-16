//! wp_presentation — precise presentation timing protocol.
//!
//! Provides precise presentation timing information to clients.
//! Browsers use this for smooth video playback and animations.

use alloc::vec::Vec;
use edgerun_encoding::byteorder::push_u32_le;

use crate::wayland::encode::*;
use crate::wayland::{ArgType, Message};

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
    push_u32_le(&mut args, tv_sec_hi);
    push_u32_le(&mut args, tv_sec_lo);
    push_u32_le(&mut args, tv_nsec);
    push_u32_le(&mut args, refresh);
    push_u32_le(&mut args, seq_hi);
    push_u32_le(&mut args, seq_lo);
    push_u32_le(&mut args, kind);
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
