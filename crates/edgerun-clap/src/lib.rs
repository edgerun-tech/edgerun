#![cfg_attr(any(not(feature = "std"), target_os = "none"), no_std)]

extern crate alloc;

pub mod derive;

pub mod cli;

pub use cli::{Arg, ArgGroup, ArgMatches, Command, FromArgMatches, Parser};

pub use edgerun_clap_derive::{arg, command, Parser, Subcommand};
