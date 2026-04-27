//! Minimal URL encoding helper — delegates to edgerun-encoding.

use crate::prelude::*;
pub use edgerun_encoding::percent::percent_encode as encode;
