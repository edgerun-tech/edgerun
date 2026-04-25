use edgerun_error::Error;

#[derive(Error, Debug)]
pub enum AcmeError {
    Network(String),
    Parse(String),
    Server(u16, Option<crate::types::AcmeErrorDetail>),
    Crypto(String),
    Storage(String),
    Protocol(String),
    NotInitialized,
    ChallengeFailed(String),
    OrderInvalid(String),
    Account(String),
}
