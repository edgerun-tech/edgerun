#![allow(dead_code)]
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
//! ```ignore
//! use edgerun_secret_service::Backend;
//!
//! let mut backend = Backend::new(data_root)?;
//! backend.put(coll_path, key, b"secret-value", "Label", &[])?;
//! let (secret, meta) = backend.get(coll_path, key)?;
//! ```

pub mod backend;
pub mod dbus_bus;
pub mod dbus_server;
pub mod dbus_types;
pub mod dbus_wire;
pub mod event_log;
pub mod session;

pub use backend::{Backend, CredentialMeta};
pub use dbus_bus::BusConnection;
pub use event_log::{EventLog, SecretEvent, SecretEventType, SECRET_STREAM_ID};
pub use session::{Session, SessionManager, BiometricVerifier, NoBiometricVerifier, DEFAULT_IDLE_TIMEOUT_US};
