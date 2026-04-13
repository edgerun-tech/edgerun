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

impl Default for RequestMode {
    fn default() -> Self {
        Self::Cors
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestCredentials {
    Omit,
    Sameorigin,
    Include,
}

impl Default for RequestCredentials {
    fn default() -> Self {
        Self::Sameorigin
    }
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

impl Default for RequestCache {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestRedirect {
    Follow,
    Error,
    Manual,
}

impl Default for RequestRedirect {
    fn default() -> Self {
        Self::Follow
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestPriority {
    High,
    Low,
    Auto,
}

impl Default for RequestPriority {
    fn default() -> Self {
        Self::Auto
    }
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

impl Default for ResponseType {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferrerPolicy {
    Noreferrer,
    NoReferrerWhenDowngrade,
    Sameorigin,
    Origin,
    StrictOrigin,
    OriginWhenCrossOrigin,
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

impl Default for ReferrerPolicy {
    fn default() -> Self {
        Self::StrictOriginWhenCrossOrigin
    }
}

impl ReferrerPolicy {
    /// Convert to its wire-format string representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Noreferrer => "no-referrer",
            Self::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            Self::Sameorigin => "same-origin",
            Self::Origin => "origin",
            Self::StrictOrigin => "strict-origin",
            Self::OriginWhenCrossOrigin => "origin-when-cross-origin",
            Self::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            Self::UnsafeUrl => "unsafe-url",
        }
    }

    /// Parse from a string value.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "no-referrer" => Some(Self::Noreferrer),
            "no-referrer-when-downgrade" => Some(Self::NoReferrerWhenDowngrade),
            "same-origin" => Some(Self::Sameorigin),
            "origin" => Some(Self::Origin),
            "strict-origin" => Some(Self::StrictOrigin),
            "origin-when-cross-origin" => Some(Self::OriginWhenCrossOrigin),
            "strict-origin-when-cross-origin" => Some(Self::StrictOriginWhenCrossOrigin),
            "unsafe-url" => Some(Self::UnsafeUrl),
            _ => None,
        }
    }
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
    pub referrer_policy: ReferrerPolicy,
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

/// HTTP response with proper header and body types.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub url: String,
    pub response_type: ResponseType,
    pub redirected: bool,
    pub body: Option<Vec<u8>>,
}

impl Response {
    /// Whether the response status is in the 200-299 range.
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}
