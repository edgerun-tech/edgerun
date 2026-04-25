use edgerun_error::Error;

#[derive(Error, Debug)]
pub enum SchedulerError {
    ProviderNotFound(String),
    NoProviderAvailable,
    DeploymentNotFound(String),
    OnChainError(String),
    ConnectionError(String),
    InvalidMetrics(String),
}
