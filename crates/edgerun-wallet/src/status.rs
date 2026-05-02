//! Canonical order status machine for EdgeRun Wallet Instant Settlement.
//!
//! Implements the status transition rules from the spec.
//! Terminal states cannot transition to non-terminal states.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalOrderStatus {
    Quoted = 1,
    QuoteExpired = 2,
    Created = 3,
    AwaitingDeposit = 4,
    DepositSeen = 5,
    DepositConfirmed = 6,
    Exchanging = 7,
    Sending = 8,
    Completed = 9,
    ActionRequired = 10,
    RefundRequired = 11,
    Refunding = 12,
    Refunded = 13,
    Expired = 14,
    Failed = 15,
    Rejected = 16,
    OnHold = 17,
    Canceled = 18,
    PartialDeposits = 19,
}

/// Terminal states — once reached, no further transitions allowed.
pub const TERMINAL_STATUSES: &[CanonicalOrderStatus] = &[
    CanonicalOrderStatus::Completed,
    CanonicalOrderStatus::Expired,
    CanonicalOrderStatus::Failed,
    CanonicalOrderStatus::Refunded,
    CanonicalOrderStatus::Rejected,
    CanonicalOrderStatus::Canceled,
];

/// Returns true if the status is terminal.
pub fn is_terminal(status: CanonicalOrderStatus) -> bool {
    TERMINAL_STATUSES.contains(&status)
}

/// Validate a state transition.
/// Returns true if the transition is allowed.
pub fn can_transition(from: CanonicalOrderStatus, to: CanonicalOrderStatus) -> bool {
    // Terminal states cannot transition to anything else
    if is_terminal(from) && from != to {
        return false;
    }

    // Allow same-state transitions (idempotent)
    if from == to {
        return true;
    }

    // Define allowed transitions
    match (from, to) {
        // From Quoted
        (CanonicalOrderStatus::Quoted, CanonicalOrderStatus::Created) => true,
        (CanonicalOrderStatus::Quoted, CanonicalOrderStatus::QuoteExpired) => true,
        (CanonicalOrderStatus::Quoted, CanonicalOrderStatus::Expired) => true,
        (CanonicalOrderStatus::Quoted, CanonicalOrderStatus::Canceled) => true,

        // From Created
        (CanonicalOrderStatus::Created, CanonicalOrderStatus::AwaitingDeposit) => true,
        (CanonicalOrderStatus::Created, CanonicalOrderStatus::Expired) => true,
        (CanonicalOrderStatus::Created, CanonicalOrderStatus::Canceled) => true,
        (CanonicalOrderStatus::Created, CanonicalOrderStatus::Rejected) => true,

        // From AwaitingDeposit
        (CanonicalOrderStatus::AwaitingDeposit, CanonicalOrderStatus::DepositSeen) => true,
        (CanonicalOrderStatus::AwaitingDeposit, CanonicalOrderStatus::Exchanging) => true, // Some providers skip ahead
        (CanonicalOrderStatus::AwaitingDeposit, CanonicalOrderStatus::Expired) => true,
        (CanonicalOrderStatus::AwaitingDeposit, CanonicalOrderStatus::Canceled) => true,
        (CanonicalOrderStatus::AwaitingDeposit, CanonicalOrderStatus::RefundRequired) => true,

        // From DepositSeen
        (CanonicalOrderStatus::DepositSeen, CanonicalOrderStatus::DepositConfirmed) => true,
        (CanonicalOrderStatus::DepositSeen, CanonicalOrderStatus::Exchanging) => true,
        (CanonicalOrderStatus::DepositSeen, CanonicalOrderStatus::Expired) => true,

        // From DepositConfirmed
        (CanonicalOrderStatus::DepositConfirmed, CanonicalOrderStatus::Exchanging) => true,
        (CanonicalOrderStatus::DepositConfirmed, CanonicalOrderStatus::Expired) => true,

        // From Exchanging
        (CanonicalOrderStatus::Exchanging, CanonicalOrderStatus::Sending) => true,
        (CanonicalOrderStatus::Exchanging, CanonicalOrderStatus::Completed) => true, // Fast path
        (CanonicalOrderStatus::Exchanging, CanonicalOrderStatus::OnHold) => true,
        (CanonicalOrderStatus::Exchanging, CanonicalOrderStatus::Failed) => true,

        // From Sending
        (CanonicalOrderStatus::Sending, CanonicalOrderStatus::Completed) => true,
        (CanonicalOrderStatus::Sending, CanonicalOrderStatus::OnHold) => true,
        (CanonicalOrderStatus::Sending, CanonicalOrderStatus::Failed) => true,

        // From ActionRequired
        (CanonicalOrderStatus::ActionRequired, CanonicalOrderStatus::Exchanging) => true,
        (CanonicalOrderStatus::ActionRequired, CanonicalOrderStatus::Sending) => true,
        (CanonicalOrderStatus::ActionRequired, CanonicalOrderStatus::Completed) => true,
        (CanonicalOrderStatus::ActionRequired, CanonicalOrderStatus::Canceled) => true,

        // From RefundRequired
        (CanonicalOrderStatus::RefundRequired, CanonicalOrderStatus::Refunding) => true,
        (CanonicalOrderStatus::RefundRequired, CanonicalOrderStatus::Canceled) => true,

        // From Refunding
        (CanonicalOrderStatus::Refunding, CanonicalOrderStatus::Refunded) => true,
        (CanonicalOrderStatus::Refunding, CanonicalOrderStatus::Failed) => true,

        // From OnHold — can resolve or fail
        (CanonicalOrderStatus::OnHold, CanonicalOrderStatus::Exchanging) => true,
        (CanonicalOrderStatus::OnHold, CanonicalOrderStatus::Sending) => true,
        (CanonicalOrderStatus::OnHold, CanonicalOrderStatus::Completed) => true,
        (CanonicalOrderStatus::OnHold, CanonicalOrderStatus::Failed) => true,
        (CanonicalOrderStatus::OnHold, CanonicalOrderStatus::Canceled) => true,

        // From PartialDeposits
        (CanonicalOrderStatus::PartialDeposits, CanonicalOrderStatus::DepositSeen) => true,
        (CanonicalOrderStatus::PartialDeposits, CanonicalOrderStatus::Exchanging) => true,
        (CanonicalOrderStatus::PartialDeposits, CanonicalOrderStatus::RefundRequired) => true,

        // All other transitions are invalid
        _ => false,
    }
}

impl fmt::Display for CanonicalOrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
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
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_states() {
        assert!(is_terminal(CanonicalOrderStatus::Completed));
        assert!(is_terminal(CanonicalOrderStatus::Expired));
        assert!(is_terminal(CanonicalOrderStatus::Failed));
        assert!(is_terminal(CanonicalOrderStatus::Refunded));
        assert!(is_terminal(CanonicalOrderStatus::Rejected));
        assert!(is_terminal(CanonicalOrderStatus::Canceled));

        assert!(!is_terminal(CanonicalOrderStatus::Created));
        assert!(!is_terminal(CanonicalOrderStatus::Exchanging));
    }

    #[test]
    fn valid_transitions() {
        assert!(can_transition(CanonicalOrderStatus::Created, CanonicalOrderStatus::AwaitingDeposit));
        assert!(can_transition(CanonicalOrderStatus::AwaitingDeposit, CanonicalOrderStatus::DepositSeen));
        assert!(can_transition(CanonicalOrderStatus::Exchanging, CanonicalOrderStatus::Sending));
        assert!(can_transition(CanonicalOrderStatus::Sending, CanonicalOrderStatus::Completed));
    }

    #[test]
    fn terminal_no_transition() {
        assert!(!can_transition(CanonicalOrderStatus::Completed, CanonicalOrderStatus::Exchanging));
        assert!(!can_transition(CanonicalOrderStatus::Failed, CanonicalOrderStatus::Exchanging));
        assert!(!can_transition(CanonicalOrderStatus::Refunded, CanonicalOrderStatus::Created));
    }

    #[test]
    fn invalid_transitions() {
        // Can't go from Created to Completed directly (must go through flow)
        assert!(!can_transition(CanonicalOrderStatus::Created, CanonicalOrderStatus::Completed));
        // Can't go from Sending back to Exchanging
        assert!(!can_transition(CanonicalOrderStatus::Sending, CanonicalOrderStatus::Exchanging));
    }

    #[test]
    fn same_state_idempotent() {
        assert!(can_transition(CanonicalOrderStatus::Exchanging, CanonicalOrderStatus::Exchanging));
        assert!(can_transition(CanonicalOrderStatus::Completed, CanonicalOrderStatus::Completed));
    }
}
