//! Solana client errors

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SolanaError {
    #[error("RPC error: {0}")]
    Rpc(String),
    
    #[error("Transaction error: {0}")]
    Transaction(String),
    
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    
    #[error("Invalid account data: {0}")]
    InvalidAccountData(String),
    
    #[error(" serialization error: {0}")]
    Serialization(String),
    
    #[error("Insufficient collateral: need {need}, have {have}")]
    InsufficientCollateral { need: u64, have: u64 },
    
    #[error("Invalid provider status: {0}")]
    InvalidStatus(String),
    
    #[error("Unauthorized")]
    Unauthorized,
    
    #[error("Deployment exhausted")]
    DeploymentExhausted,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Signing error: {0}")]
    Signing(String),
}