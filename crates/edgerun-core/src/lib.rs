#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "conformance")]
#[allow(unused_macros)]
macro_rules! println {
    ($($arg:tt)*) => {
        std::println!($($arg)*)
    };
}

#[cfg(feature = "conformance")]
#[allow(unused_macros)]
macro_rules! eprintln {
    ($($arg:tt)*) => {
        std::eprintln!($($arg)*)
    };
}

#[cfg(not(feature = "conformance"))]
#[allow(unused_macros)]
macro_rules! println {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "conformance"))]
#[allow(unused_macros)]
macro_rules! eprintln {
    ($($arg:tt)*) => {};
}

/// Core no_std prelude for edgerun-core internals.
///
/// This is intentionally alloc/core only. It is not a fake std shim and does
/// not expose filesystem, process, environment, wall-clock, or host I/O APIs.
pub mod prelude {
    pub mod v1 {
        pub use alloc::borrow::ToOwned;
        pub use alloc::boxed::Box;
        pub use alloc::collections::{BTreeMap, BTreeSet};
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
    }
}

/// Alloc-only collection aliases used by existing core validators.
///
/// This is an explicit no_std boundary, not a `std` compatibility layer.
/// `HashMap`/`HashSet` are deterministic BTree-backed aliases until a real
/// hash dependency is introduced deliberately.
pub mod collections {
    pub use alloc::collections::{BTreeMap, BTreeSet};

    pub type HashMap<K, V> = BTreeMap<K, V>;
    pub type HashSet<T> = BTreeSet<T>;
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
