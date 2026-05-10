#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "derive")]
pub use edgerun_error_derive::Error;

#[cfg(feature = "anyhow-compat")]
pub use anyhow::Context;
#[cfg(feature = "anyhow-compat")]
pub use anyhow::Error;
#[cfg(feature = "anyhow-compat")]
pub use anyhow::Error as Report;
#[cfg(feature = "anyhow-compat")]
pub use anyhow::Result;
#[cfg(feature = "anyhow-compat")]
pub use anyhow::anyhow;
#[cfg(feature = "anyhow-compat")]
pub use anyhow::bail;
#[cfg(feature = "anyhow-compat")]
pub use anyhow::ensure;

#[cfg(feature = "anyhow-compat")]
pub mod anyhow_compat {
    pub use anyhow::*;
}

#[cfg(not(feature = "anyhow-compat"))]
pub type Result<T, E> = core::result::Result<T, E>;
