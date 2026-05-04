//! Provider mapping — map provider status strings to canonical statuses.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderStatusMapping {
    pub provider_code: &'static str,
    pub provider_status: &'static str,
    pub canonical: i32, // CanonicalOrderStatus as i32
}

/// Map a provider-specific status string to canonical status.
/// Unknown statuses map to ON_HOLD.
pub fn map_provider_status(provider_code: &str, provider_status: &str) -> i32 {
    // SideShift status mappings
    let sideshift_mappings: &[(&str, i32)] = &[
        ("pending", 3),           // CREATED
        ("awaiting_deposit", 4),  // AWAITING_DEPOSIT
        ("deposit_seen", 5),      // DEPOSIT_SEEN
        ("deposit_confirmed", 6), // DEPOSIT_CONFIRMED
        ("exchanging", 7),        // EXCHANGING
        ("sending", 8),           // SENDING
        ("completed", 9),         // COMPLETED
        ("failed", 15),           // FAILED
        ("refund_required", 11),  // REFUND_REQUIRED
        ("refunding", 12),        // REFUNDING
        ("refunded", 13),         // REFUNDED
        ("expired", 14),          // EXPIRED
        ("on_hold", 17),          // ON_HOLD
    ];

    // ChangeNOW status mappings
    let changenow_mappings: &[(&str, i32)] = &[
        ("new", 3),        // CREATED
        ("waiting", 4),    // AWAITING_DEPOSIT
        ("confirming", 5), // DEPOSIT_SEEN
        ("exchanging", 7), // EXCHANGING
        ("sending", 8),    // SENDING
        ("finished", 9),   // COMPLETED
        ("failed", 15),    // FAILED
        ("refunded", 13),  // REFUNDED
        ("expired", 14),   // EXPIRED
    ];

    let mappings: &[(&str, i32)] = match provider_code {
        "SIDESHIFT" => sideshift_mappings,
        "CHANGENOW" => changenow_mappings,
        _ => &[],
    };

    for (status_str, canonical) in mappings {
        if *status_str == provider_status {
            return *canonical;
        }
    }

    // Unknown status -> ON_HOLD
    17 // ON_HOLD
}

impl fmt::Display for ProviderStatusMapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{} => {}",
            self.provider_code, self.provider_status, self.canonical
        )
    }
}
