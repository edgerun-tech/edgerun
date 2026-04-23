use solana_program::program_error::ProgramError;

#[derive(Debug, Clone, Copy)]
pub enum RegistryError {
    InsufficientCollateral,
    Unauthorized,
    ProviderNotFound,
    InvalidState,
    Overflow,
}

impl From<RegistryError> for ProgramError {
    fn from(e: RegistryError) -> Self {
        match e {
            RegistryError::InsufficientCollateral => {
                solana_program::msg!("Error: Insufficient collateral");
                ProgramError::Custom(1)
            }
            RegistryError::Unauthorized => {
                solana_program::msg!("Error: Unauthorized");
                ProgramError::Custom(2)
            }
            RegistryError::ProviderNotFound => {
                solana_program::msg!("Error: Provider not found");
                ProgramError::Custom(3)
            }
            RegistryError::InvalidState => {
                solana_program::msg!("Error: Invalid state");
                ProgramError::Custom(4)
            }
            RegistryError::Overflow => {
                solana_program::msg!("Error: Arithmetic overflow");
                ProgramError::Custom(5)
            }
        }
    }
}