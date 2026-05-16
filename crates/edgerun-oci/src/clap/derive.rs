pub trait Parser: Sized {
    fn command() -> crate::clap::cli::Command;
    fn from(matches: &crate::clap::cli::ArgMatches) -> Self;
}
