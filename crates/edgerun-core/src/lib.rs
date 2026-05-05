#![no_std]

extern crate alloc;

#[cfg(all(not(target_os = "none"), feature = "conformance"))]
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

#[cfg(any(target_os = "none", not(feature = "conformance")))]
macro_rules! println {
    ($($arg:tt)*) => {};
}

#[cfg(any(target_os = "none", not(feature = "conformance")))]
macro_rules! eprintln {
    ($($arg:tt)*) => {};
}

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
pub mod wire_boundary;
pub mod wire_boundary_free;
pub mod wire_command;
pub mod wire_network;
pub mod wire_stream;
pub mod wire_trust;
