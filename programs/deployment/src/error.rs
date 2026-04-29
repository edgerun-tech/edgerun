use solana_program::program_error::ProgramError;

#[derive(Debug, Clone, Copy)]
pub enum DeploymentError {
    Unauthorized,
    ZeroDeposit,
    InvalidState,
    DepositExhausted,
    Overflow,
    NotRunning,
    PriceChangeGracePeriod,
    ProviderNotAssigned,
    SelfProviderNotAllowed,
}

impl From<DeploymentError> for ProgramError {
    fn from(e: DeploymentError) -> Self {
        match e {
            DeploymentError::Unauthorized => {
                solana_program::msg!("Error: Unauthorized");
                ProgramError::Custom(10)
            }
            DeploymentError::ZeroDeposit => {
                solana_program::msg!("Error: Zero deposit");
                ProgramError::Custom(11)
            }
            DeploymentError::InvalidState => {
                solana_program::msg!("Error: Invalid state for operation");
                ProgramError::Custom(12)
            }
            DeploymentError::DepositExhausted => {
                solana_program::msg!("Error: Deposit exhausted");
                ProgramError::Custom(13)
            }
            DeploymentError::Overflow => {
                solana_program::msg!("Error: Arithmetic overflow");
                ProgramError::Custom(14)
            }
            DeploymentError::NotRunning => {
                solana_program::msg!("Error: Deployment not running");
                ProgramError::Custom(15)
            }
            DeploymentError::PriceChangeGracePeriod => {
                solana_program::msg!("Error: price changes require at least 24 hours grace");
                ProgramError::Custom(16)
            }
            DeploymentError::ProviderNotAssigned => {
                solana_program::msg!("Error: provider not assigned");
                ProgramError::Custom(17)
            }
            DeploymentError::SelfProviderNotAllowed => {
                solana_program::msg!("Error: buyer cannot assign their own provider account");
                ProgramError::Custom(18)
            }
        }
    }
}
