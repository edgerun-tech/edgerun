//! Error handling - built-in Error trait and utilities.


extern crate alloc;

pub trait Error: core::fmt::Display + core::fmt::Debug {
    fn description(&self) -> &str {
        ""
    }
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}