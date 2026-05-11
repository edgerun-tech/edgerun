//! Deprecated admission runtime v1.
//!
//! This file intentionally contains no implementation. The maintained admission
//! runtime is `admission_v2.rs`, which verifies signed user work requests and
//! uses first-class replay state. Do not re-enable this module.

compile_error!("stale admission_v1.rs is deprecated; use std_runtime/admission_v2.rs");
