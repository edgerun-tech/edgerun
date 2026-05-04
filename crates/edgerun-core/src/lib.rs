#![no_std]

extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(all(not(target_os = "none"), feature = "conformance"))]
macro_rules! println {
    ($($arg:tt)*) => {
        std::println!($($arg)*)
    };
}

#[cfg(all(not(target_os = "none"), feature = "conformance"))]
macro_rules! eprintln {
    ($($arg:tt)*) => {
        std::eprintln!($($arg)*)
    };
}

#[cfg(target_os = "none")]
macro_rules! println {
    ($($arg:tt)*) => {};
}

#[cfg(all(target_os = "none", feature = "conformance"))]
macro_rules! eprintln {
    ($($arg:tt)*) => {};
}

#[cfg(target_os = "none")]
extern crate self as std;

#[path = "std.rs"]
mod std_compat;
pub use std_compat::*;

pub mod command;
#[cfg(feature = "conformance")]
pub mod conformance;
pub mod crypto;
pub mod fixed_point;
#[path = "protocol_native/mod.rs"]
pub mod protocol;
pub mod result;
pub mod util;
pub mod validators;
pub mod value;
pub mod varint;
#[cfg(feature = "proto-compat")]
pub mod wire_boundary;
pub mod wire_command;
pub mod wire_stream;
