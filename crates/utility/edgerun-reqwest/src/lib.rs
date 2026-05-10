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
