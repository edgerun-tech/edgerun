//! Edgerun-owned compatibility surface for Reqwest-shaped HTTP client APIs.
//!
//! The crate root currently forwards Reqwest's API so Codex can move imports
//! behind an Edgerun-owned boundary without losing behavior. Native Edgerun
//! HTTP building blocks are exposed under `edgerun` for progressive internal
//! replacement.

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

#[cfg(feature = "native-root")]
pub use native::*;

#[cfg(all(feature = "compat", not(feature = "native-root")))]
pub use reqwest::*;
