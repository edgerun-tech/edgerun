#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod derive;

#[cfg(feature = "std")]
pub mod cli;

#[cfg(feature = "std")]
pub use cli::{Arg, ArgGroup, ArgMatches, Command, FromArgMatches, Parser};