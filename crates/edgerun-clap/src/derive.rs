#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;

pub trait Parser: Sized {
    fn command() -> crate::cli::Command;
    fn from(matches: &crate::cli::ArgMatches) -> Self;
}

#[cfg(feature = "derive")]
pub use edgerun_clap_derive::Parser;

pub mod macros {
    macro_rules! Parser {
        () => {
            #[derive(Debug, serde::Deserialize)]
            #[serde(rename_all = "kebab-case")]
            pub struct Cli {
                #[serde(skip)]
                pub command: String,
            }
        };
    }

    macro_rules! command {
        ($name:expr) => {
            $crate::cli::Command::new($name)
        };
        ($name:expr, $($tt:tt)*) => {
            $crate::cli::Command::new($name)
        };
    }

    macro_rules! arg {
        ($name:expr) => {
            $crate::cli::Arg::new($name)
        };
        ($name:expr, short($s:literal)) => {
            $crate::cli::Arg::new($name).short($s)
        };
        ($name:expr, long($l:literal)) => {
            $crate::cli::Arg::new($name).long($l)
        };
        ($name:expr, help($h:literal)) => {
            $crate::cli::Arg::new($name).help($h)
        };
        ($name:expr, default_value($v:literal)) => {
            $crate::cli::Arg::new($name).default_value($v)
        };
    }
}