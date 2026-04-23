use thiserror::Error;

#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    
    #[error("No provider available for resources")]
    NoProviderAvailable,
    
    #[error("Deployment not found: {0}")]
    DeploymentNotFound(String),
    
    #[error("On-chain error: {0}")]
    OnChainError(String),
    
    #[error("Provider connection error: {0}")]
    ConnectionError(String),
    
    #[error("Invalid metrics: {0}")]
    InvalidMetrics(String),
}