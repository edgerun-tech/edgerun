//! Fetch types — from WHATWG Fetch Living Standard.
//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform.py
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{string::String, vec::Vec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestMode {
    Sameorigin,
    Cors,
    Nocors,
    Navigate,
    Websocket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestCredentials {
    Omit,
    Sameorigin,
    Include,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestCache {
    Default,
    Nostore,
    Reload,
    Nocache,
    Forcecache,
    Onlyifcached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestRedirect {
    Follow,
    Error,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestPriority {
    High,
    Low,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    Basic,
    Cors,
    Default,
    Error,
    Opaque,
    Opaqueredirect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferrerPolicy {
    Noreferrer,
    Noreferrerwhendowngrade,
    Sameorigin,
    Origin,
    Strictorigin,
    Originwhencrossorigin,
    Strictoriginwhencrossorigin,
    Unsafeurl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestDestination {
    Audio,
    Audioworklet,
    Document,
    Embed,
    Font,
    Frame,
    Iframe,
    Image,
    Manifest,
    Object,
    Painterworklet,
    Report,
    Script,
    Sharedworker,
    Style,
    Track,
    Video,
    Worker,
    Xslt,
}

#[derive(Debug, Clone)]
pub struct RequestInit {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub referrer: String,
    pub referrer_policy: String,
    pub mode: RequestMode,
    pub credentials: RequestCredentials,
    pub cache: RequestCache,
    pub redirect: RequestRedirect,
    pub integrity: String,
    pub keep_alive: bool,
    pub signal: Option<()>,
    pub priority: RequestPriority,
    pub duplex: String,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub ok: bool,
    pub status: u16,
    pub status_text: String,
    pub headers: (),
    pub url: String,
    pub response_type: ResponseType,
    pub redirected: bool,
    pub body: Option<Option<Vec<u8>>>,
}
