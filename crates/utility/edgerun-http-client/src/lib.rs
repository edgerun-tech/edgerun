//! EdgeRun HTTP client split out from the node orchestration crate.

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

pub mod rt {
    pub use edgerun_runtime::rt::*;
}

#[cfg(feature = "tls")]
pub mod tls;

pub mod http;

#[cfg(feature = "client")]
pub use http::{HttpClient, HttpVersion};
