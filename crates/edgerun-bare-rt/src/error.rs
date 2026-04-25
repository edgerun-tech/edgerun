//! Error handling - built-in Error trait and utilities.

#![no_std]

extern crate alloc;

pub trait Error: core::fmt::Display + core::fmt::Debug {
    fn description(&self) -> &str {
        ""
    }
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}