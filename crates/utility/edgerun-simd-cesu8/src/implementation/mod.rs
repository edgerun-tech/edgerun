//! This module contains certain implementation details that are used for
//! optimization purposes.
//!
//! THIS MODULE IS NOT PART OF THE PUBLIC API AND IS SEMVER EXEMPT.

pub mod fallback;
#[cfg(any())]
pub mod simd;
#[cfg(any(feature = "bench", not(feature = "nightly"), feature = "nightly"))]
pub mod word;

#[cfg(any())]
pub use self::simd as active;
#[cfg(any(not(feature = "nightly"), feature = "nightly"))]
pub use self::word as active;
