pub trait Parser: Sized {
    fn command() -> crate::cli::Command;
    fn from(matches: &crate::cli::ArgMatches) -> Self;
}

#[cfg(feature = "derive")]
pub use edgerun_clap_derive::Parser;
