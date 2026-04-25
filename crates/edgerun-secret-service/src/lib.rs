//! Secret service library.
//!
//! Provides a backend for storing and retrieving secrets via encrypted
//! blob storage, with an append-only event log for audit history.
//!
//! ## Architecture
//!
//! - **Backend** — BlobStore + FileIndex + EventLog for credential CRUD
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
//! let mut backend = Backend::new(tmp.clone())?;
//! let coll = "/org/freedesktop/secrets/collections/default";
//! backend.put(coll, "doc-key", b"secret-value", "Doc Label", &[])?;
//! let (secret, meta) = backend.get(coll, "doc-key")?.unwrap();
//! assert_eq!(secret, b"secret-value");
//! assert_eq!(meta.label, "Doc Label");
//! # let _ = std::fs::remove_dir_all(&tmp);
//! # Ok(())
//! # }
//! ```

pub mod backend;
pub mod dbus_bus;
pub mod dbus_server;
pub mod dbus_types;
pub mod dbus_wire;
pub mod event_log;
pub mod session;

pub use backend::{no_op_event_recorder, Backend, CredentialMeta, SecretEventRecorder};
pub use dbus_bus::BusConnection;
pub use event_log::{EventLog, SecretEvent, SecretEventType, SECRET_STREAM_ID};
pub use session::{
    BiometricVerifier, NoBiometricVerifier, Session, SessionManager, DEFAULT_IDLE_TIMEOUT_US,
};
