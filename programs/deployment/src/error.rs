use solana_program::program_error::ProgramError;

#[derive(Debug, Clone, Copy)]
pub enum DeploymentError {
    Unauthorized,
    ZeroDeposit,
    InvalidState,
    DepositExhausted,
    Overflow,
    NotRunning,
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
        }
    }
}
