//! xdg-activation-v1 protocol — window activation/attention requests.
//! 
//! This protocol allows clients to request user attention for a specific window.
//! Browsers use this for tab notifications and activation requests.

use crate::wire::{ArgType, Message};
use crate::wire::encode::*;

pub const ZXDG_ACTIVATION_V1: &str = "xdg_activation_v1";
pub const ZXDG_ACTIVATION_V1_VERSION: u32 = 1;

pub mod xdg_activation_request {
    use super::*;

    pub const DESTROY: u16 = 0;
    pub const DESTROY_SIG: &[ArgType] = &[];

    pub const GET_ACTIVATION_TOKEN: u16 = 1;
    pub const GET_ACTIVATION_TOKEN_SIG: &[ArgType] = &[ArgType::NewId];

    pub const ACTIVATE: u16 = 2;
    pub const ACTIVATE_SIG: &[ArgType] = &[ArgType::String, ArgType::Object];
}

pub mod xdg_activation_event {
    /// The compositor has generated an activation token.
    pub const DONE: u16 = 0;
    // sig: string (token)
}

/// Build the done event with an activation token.
pub fn activation_done_event(token_obj_id: u32, token: &str) -> Message {
    message_uint_string(token_obj_id, xdg_activation_event::DONE, 0, token)
}

/// Build destroy request (client -> compositor, no response needed).
pub fn activation_destroy_request(activation_id: u32) -> Message {
    message_empty(activation_id, xdg_activation_request::DESTROY)
}
