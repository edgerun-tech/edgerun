//! Secret service library.
//!
//! Provides a backend for storing and retrieving secrets via encrypted
//! blob storage, with an append-only event log for audit history.
//!
//! ## Architecture
//!
//! - **Backend** — BlobStore + FileIndex for credential CRUD
//! - **Session** — Biometric verification state with idle auto-lock
//! - **D-Bus server** — Freedesktop Secret Service compatible API over Unix socket
//!
//! ## Using the Backend
//!
//! ```
//! use std::path::PathBuf;
//! use edgerun_secret_service::Backend;
//!
//! # fn main() -> std::io::Result<()> {
//! let tmp = std::env::temp_dir().join("ss_doc_test");
//! let _ = std::fs::remove_dir_all(&tmp);
//! let mut backend = Backend::new(tmp.clone(), Box::new(|_, _| Ok(())))?;
//! let coll = "/org/freedesktop/secrets/collections/default";
//! backend.put(coll, "doc-key", b"secret-value", "Doc Label", &[])?;
//! let (secret, meta) = backend.get(coll, "doc-key")?.unwrap();
//! assert_eq!(secret, b"secret-value");
//! assert_eq!(meta.label, "Doc Label");
//! # let _ = std::fs::remove_dir_all(&tmp);
//! # Ok(())
//! # }
//! ```

#![no_std]

extern crate alloc;
#[cfg(target_os = "none")]
extern crate self as std;
#[cfg(not(target_os = "none"))]
extern crate std;

pub mod prelude {
    pub mod v1 {
        pub use alloc::borrow::ToOwned;
        pub use alloc::boxed::Box;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
    }
}

pub mod boxed {
    pub use alloc::boxed::Box;
}

pub mod string {
    pub use alloc::string::{String, ToString};
}

pub mod vec {
    pub use alloc::vec::Vec;
}

pub mod iter {
    pub use core::iter::*;
}

#[cfg(target_os = "none")]
pub use edgerun_storage::{
    cmp, collections, convert, env, error, ffi, fmt, format, fs, io, mem, option, os, path,
    process, result, slice, str, sync, time,
};

pub mod backend;
#[cfg(not(target_os = "none"))]
pub mod dbus_bus;
#[cfg(not(target_os = "none"))]
pub mod dbus_server;
pub mod dbus_types;
pub mod dbus_wire;
pub mod session;

pub use backend::{no_op_event_recorder, Backend, CredentialMeta, SecretEventRecorder};
#[cfg(not(target_os = "none"))]
pub use dbus_bus::BusConnection;
pub use session::{
    BiometricVerifier, NoBiometricVerifier, Session, SessionManager, DEFAULT_IDLE_TIMEOUT_US,
};
