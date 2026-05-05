#![no_std]

extern crate alloc;
#[cfg(all(feature = "std", not(target_os = "none")))]
extern crate std;

pub mod derive;

pub mod cli;

pub use cli::{Arg, ArgGroup, ArgMatches, Command, FromArgMatches, Parser};

pub use edgerun_clap_derive::{arg, command, Parser, Subcommand};
