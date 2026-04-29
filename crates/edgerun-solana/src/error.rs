//! Solana client errors

use crate::prelude::*;
use edgerun_error::Error;

#[derive(Error, Debug)]
pub enum SolanaError {
    #[error("RPC error: {0}")]
    Rpc(String),
    #[error("transaction error: {0}")]
    Transaction(String),
    #[error("account not found: {0}")]
    AccountNotFound(String),
    #[error("invalid account data: {0}")]
    InvalidAccountData(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("insufficient collateral")]
    InsufficientCollateral,
    #[error("invalid status: {0}")]
    InvalidStatus(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("deployment exhausted")]
    DeploymentExhausted,
    #[error("I/O error: {0}")]
    Io(String),
    #[error("signing error: {0}")]
    Signing(String),
}
