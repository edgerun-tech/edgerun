//! EdgeRun-owned async runtime primitives.

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

pub mod rt;
