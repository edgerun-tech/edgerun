//! Node-owned error boundary.
//!
//! Domain crates keep their local error enums. The node converts them at IPC,
//! routing, service, storage, and stream boundaries so failures are reported in
//! one place instead of becoming ad-hoc strings in leaf crates.

use alloc::string::{String, ToString};
use core::fmt;

use edgerun_storage::StorageError;

#[derive(Debug)]
pub enum NodeError {
    MissingField(String),
    CommandRejected(String),
    CommandDeferred(String),
    Stream(edgerun_stream::StreamError),
    Storage(String),
    Runtime(String),
    Service {
        service: &'static str,
        message: String,
    },
    Protocol {
        protocol: &'static str,
        message: String,
    },
}

pub type NodeResult<T> = Result<T, NodeError>;

impl NodeError {
    pub fn service(service: &'static str, error: impl fmt::Display) -> Self {
        Self::Service {
            service,
            message: error.to_string(),
        }
    }

    pub fn protocol(protocol: &'static str, error: impl fmt::Display) -> Self {
        Self::Protocol {
            protocol,
            message: error.to_string(),
        }
    }

    pub fn runtime(error: impl fmt::Display) -> Self {
        Self::Runtime(error.to_string())
    }

    pub fn log(&self) {
        crate::logging::error("edgerun_node::error", format_args!("{self}"));
    }
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeError::MissingField(field) => write!(f, "missing required field: {field}"),
            NodeError::CommandRejected(reason) => write!(f, "command rejected: {reason}"),
            NodeError::CommandDeferred(reason) => write!(f, "command deferred: {reason}"),
            NodeError::Stream(error) => write!(f, "stream error: {error}"),
            NodeError::Storage(error) => write!(f, "storage error: {error}"),
            NodeError::Runtime(error) => write!(f, "runtime error: {error}"),
            NodeError::Service { service, message } => {
                write!(f, "{service} service error: {message}")
            }
            NodeError::Protocol { protocol, message } => {
                write!(f, "{protocol} protocol error: {message}")
            }
        }
    }
}

impl core::error::Error for NodeError {}

impl From<edgerun_stream::StreamError> for NodeError {
    fn from(error: edgerun_stream::StreamError) -> Self {
        NodeError::Stream(error)
    }
}

impl From<StorageError> for NodeError {
    fn from(error: StorageError) -> Self {
        NodeError::Storage(error.to_string())
    }
}

pub fn log_boundary_error(boundary: &'static str, error: impl fmt::Display) {
    crate::logging::error(boundary, format_args!("{error}"));
}

pub fn log_boundary_warning(boundary: &'static str, warning: impl fmt::Display) {
    crate::logging::warn(boundary, format_args!("{warning}"));
}
