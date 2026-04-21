#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub trait Parser: Sized {
    fn command() -> crate::cli::Command;
    fn from(matches: &crate::cli::ArgMatches) -> Self;
}

#[cfg(feature = "derive")]
pub use edgerun_clap_derive::Parser;