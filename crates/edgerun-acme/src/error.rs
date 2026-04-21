use thiserror::Error;

#[derive(Error, Debug)]
pub enum AcmeError {
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("ACME server error: {0}")]
    Server(u16, Option<crate::types::AcmeErrorDetail>),
    
    #[error("Crypto error: {0}")]
    Crypto(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Not initialized")]
    NotInitialized,
    
    #[error("Challenge failed: {0}")]
    ChallengeFailed(String),
    
    #[error("Order invalid: {0}")]
    OrderInvalid(String),
    
    #[error("Account error: {0}")]
    Account(String),
}
