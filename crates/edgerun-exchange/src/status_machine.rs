//! Status machine for exchange orders.
//!
//! Wraps the wallet status machine with order-specific logic.
//! Order status is derived from events, not from mutable provider status.

use edgerun_wallet::{CanonicalOrderStatus, is_terminal, can_transition, WalletError};

/// Status machine for exchange orders.
#[derive(Debug, Default)]
pub struct StatusMachine;

impl StatusMachine {
    /// Validate a status transition with detailed error.
    pub fn validate_transition(
        &self,
        from: CanonicalOrderStatus,
        to: CanonicalOrderStatus,
    ) -> Result<(), WalletError> {
        if can_transition(from, to) {
            Ok(())
        } else {
            Err(WalletError::InvalidStatusTransition {
                from: from_status_str(from),
                to: from_status_str(to),
            })
        }
    }

    /// Check if status is terminal.
    pub fn is_terminal(&self, status: CanonicalOrderStatus) -> bool {
        is_terminal(status)
    }

    /// Handle provider status that contradicts a terminal state.
    /// When a provider reports a non-terminal status after we already
    /// recorded a terminal state, the order goes to OnHold for manual review.
    pub fn handle_provider_contradiction(
        &self,
        canonical: CanonicalOrderStatus,
        provider_status: CanonicalOrderStatus,
    ) -> CanonicalOrderStatus {
        if is_terminal(canonical) && canonical != provider_status {
            CanonicalOrderStatus::OnHold
        } else if is_terminal(canonical) && !can_transition(canonical, provider_status) {
            CanonicalOrderStatus::OnHold
        } else {
            provider_status
        }
    }
}

fn from_status_str(s: CanonicalOrderStatus) -> &'static str {
    match s {
        CanonicalOrderStatus::Quoted => "QUOTED",
        CanonicalOrderStatus::QuoteExpired => "QUOTE_EXPIRED",
        CanonicalOrderStatus::Created => "CREATED",
        CanonicalOrderStatus::AwaitingDeposit => "AWAITING_DEPOSIT",
        CanonicalOrderStatus::DepositSeen => "DEPOSIT_SEEN",
        CanonicalOrderStatus::DepositConfirmed => "DEPOSIT_CONFIRMED",
        CanonicalOrderStatus::Exchanging => "EXCHANGING",
        CanonicalOrderStatus::Sending => "SENDING",
        CanonicalOrderStatus::Completed => "COMPLETED",
        CanonicalOrderStatus::ActionRequired => "ACTION_REQUIRED",
        CanonicalOrderStatus::RefundRequired => "REFUND_REQUIRED",
        CanonicalOrderStatus::Refunding => "REFUNDING",
        CanonicalOrderStatus::Refunded => "REFUNDED",
        CanonicalOrderStatus::Expired => "EXPIRED",
        CanonicalOrderStatus::Failed => "FAILED",
        CanonicalOrderStatus::Rejected => "REJECTED",
        CanonicalOrderStatus::OnHold => "ON_HOLD",
        CanonicalOrderStatus::Canceled => "CANCELED",
        CanonicalOrderStatus::PartialDeposits => "PARTIAL_DEPOSITS",
    }
}
