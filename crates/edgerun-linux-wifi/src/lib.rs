#![no_std]

extern crate alloc;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;
