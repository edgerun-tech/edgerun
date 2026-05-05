//! Removed session structural wire codec.
//!
//! Session handshake payloads must use the rkyv boundary. This file is a
//! deliberate breakpoint for remaining session code that still depends on the
//! old structural wire implementation.
