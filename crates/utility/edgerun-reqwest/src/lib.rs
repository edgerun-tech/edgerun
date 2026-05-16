//! Edgerun-owned compatibility surface for Reqwest-shaped HTTP client APIs.
//!
//! The crate root exposes a Reqwest-shaped API backed by Edgerun HTTP building
//! blocks. Native Edgerun building blocks are also exposed under `edgerun` for
//! progressive internal replacement.

pub mod edgerun {
    pub mod encoding {
        pub use edgerun_encoding::*;
    }

    pub mod http {
        pub use edgerun_http::*;
    }

    pub mod json {
        pub use edgerun_json::*;
    }

    pub mod url {
        pub use edgerun_url::*;
    }
}

#[cfg(feature = "native")]
pub mod native;

#[cfg(feature = "native")]
pub use native::*;

#[cfg(not(feature = "native"))]
pub use portable::Error;

#[cfg(not(feature = "native"))]
mod portable {
    use edgerun_http::StatusCode;
    use std::fmt;

    #[derive(Debug, Clone)]
    pub struct Error {
        message: String,
        status: Option<StatusCode>,
    }

    impl Error {
        pub fn new(message: impl Into<String>) -> Self {
            Self {
                message: message.into(),
                status: None,
            }
        }

        pub fn with_status(message: impl Into<String>, status: StatusCode) -> Self {
            Self {
                message: message.into(),
                status: Some(status),
            }
        }

        pub fn status(&self) -> Option<StatusCode> {
            self.status
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.message)
        }
    }

    impl std::error::Error for Error {}
}
