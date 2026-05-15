#![no_std]

extern crate alloc;

use alloc::boxed::Box;

pub type Error = Box<dyn core::error::Error + Send + Sync + 'static>;
pub type Result<T, E = Error> = core::result::Result<T, E>;
