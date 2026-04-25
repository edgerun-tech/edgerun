//! Solana client errors

use edgerun_error::Error;

#[derive(Error, Debug)]
pub enum SolanaError {
    Rpc(String),
    Transaction(String),
    AccountNotFound(String),
    InvalidAccountData(String),
    Serialization(String),
    InsufficientCollateral,
    InvalidStatus(String),
    Unauthorized,
    DeploymentExhausted,
    Io(String),
    Signing(String),
}
