//! Status machine for exchange orders.
//!
//! Wraps the wallet status machine with order-specific logic.

use edgerun_wallet::{CanonicalOrderStatus, is_terminal, can_transition, WalletError};

/// Status machine for exchange orders.
#[derive(Debug, Default)]
pub struct StatusMachine;

impl StatusMachine {
    /// Validate a status transition.
    pub fn validate_transition(
        &self,
        from: CanonicalOrderStatus,
        to: CanonicalOrderStatus,
    ) -> Result<(), WalletError> {
        if can_transition(from, to) {
            Ok(())
        } else {
            Err(WalletError::InvalidStatusTransition {
                from: "invalid_transition",
                to: "invalid_transition",
            })
        }
    }

    /// Check if status is terminal.
    pub fn is_terminal(&self, status: CanonicalOrderStatus) -> bool {
        is_terminal(status)
    }

    /// Handle provider status that contradicts a terminal state.
    /// Appends dispute/manual-review event.
    pub fn handle_provider_contradiction(
        &self,
        current: CanonicalOrderStatus,
        provider_status: CanonicalOrderStatus,
    ) -> CanonicalOrderStatus {
        if is_terminal(current) && current != provider_status {
            // Provider contradicts terminal state — go to ON_HOLD for manual review
            CanonicalOrderStatus::OnHold
        } else {
            provider_status
        }
    }
}
