#![no_std]

//! EdgeRun marketplace domain model.
//!
//! This crate does not own stream authority, settlement custody, or HTTP
//! ingress. It defines rkyv-native marketplace facts that callers can commit
//! through the node stream and project back into browse, checkout, and install
//! state.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use edgerun_core::protocol::{CapabilityDescriptor, Digest, IdentityRef, NodeRef, ObjectRef};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarketplaceError {
    MissingField(&'static str),
    InvalidFeePolicy(&'static str),
    InvalidStatus(&'static str),
    NotFound(&'static str),
    NotPaid,
    ListingNotActive,
    PackageMismatch,
}

impl core::fmt::Display for MarketplaceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "missing marketplace field: {field}"),
            Self::InvalidFeePolicy(reason) => write!(f, "invalid marketplace fee policy: {reason}"),
            Self::InvalidStatus(reason) => write!(f, "invalid marketplace status: {reason}"),
            Self::NotFound(entity) => write!(f, "marketplace entity not found: {entity}"),
            Self::NotPaid => f.write_str("marketplace checkout is not paid"),
            Self::ListingNotActive => f.write_str("marketplace listing is not active"),
            Self::PackageMismatch => f.write_str("marketplace package mismatch"),
        }
    }
}

impl core::error::Error for MarketplaceError {}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
#[repr(i32)]
pub enum ListingStatus {
    Draft = 0,
    Active = 1,
    Suspended = 2,
    Expired = 3,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
#[repr(i32)]
pub enum CheckoutStatus {
    Created = 0,
    Quoted = 1,
    OrderOpened = 2,
    Paid = 3,
    Failed = 4,
    Expired = 5,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
pub struct SettlementAddress {
    pub asset_id: String,
    pub address: String,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
pub struct MarketplaceFeePolicy {
    pub edgerun_bps: u32,
    pub app_bps: u32,
    pub max_total_bps: u32,
    pub edgerun_recipient: Option<SettlementAddress>,
    pub app_recipient: Option<SettlementAddress>,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
pub struct MarketplacePublisher {
    pub publisher_id: String,
    pub identity_ref: Option<IdentityRef>,
    pub display_name: String,
    pub settlement_addresses: Vec<SettlementAddress>,
    pub signing_keys: Vec<Vec<u8>>,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
pub struct MarketplacePackage {
    pub package_id: String,
    pub app_package_object: ObjectRef,
    pub publisher_id: String,
    pub version: String,
    pub content_hash: Digest,
    pub required_capabilities: Vec<CapabilityDescriptor>,
    pub media_objects: Vec<ObjectRef>,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
pub struct MarketplaceListing {
    pub listing_id: String,
    pub package_id: String,
    pub publisher_id: String,
    pub price_asset: String,
    pub price_amount: String,
    pub seller_settlement_asset: String,
    pub seller_settlement_address: String,
    pub fee_policy: MarketplaceFeePolicy,
    pub status: i32,
    pub expires_at_ms: u64,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
pub struct MarketplaceCheckout {
    pub checkout_id: String,
    pub listing_id: String,
    pub buyer_node: Option<NodeRef>,
    pub install_target_node: Option<NodeRef>,
    pub payment_request_id: String,
    pub quote_id: String,
    pub order_id: String,
    pub receipt_id: String,
    pub status: i32,
    pub fee_policy: MarketplaceFeePolicy,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
pub struct MarketplaceInstallReceipt {
    pub listing_id: String,
    pub package_id: String,
    pub receipt_id: String,
    pub install_command_id: Vec<u8>,
    pub installed_event_id: Vec<u8>,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
pub enum MarketplaceEvent {
    PublisherRegistered(MarketplacePublisher),
    PackagePublished(MarketplacePackage),
    ListingPublished(MarketplaceListing),
    ListingSuspended {
        listing_id: String,
        reason: String,
        suspended_at_ms: u64,
    },
    CheckoutCreated(MarketplaceCheckout),
    CheckoutPaid {
        checkout_id: String,
        receipt_id: String,
        paid_at_ms: u64,
    },
    InstallAuthorized {
        checkout_id: String,
        package_id: String,
        install_command_id: Vec<u8>,
        authorized_at_ms: u64,
    },
    PackageInstalled(MarketplaceInstallReceipt),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarketplaceProjection {
    pub publishers: BTreeMap<String, MarketplacePublisher>,
    pub packages: BTreeMap<String, MarketplacePackage>,
    pub listings: BTreeMap<String, MarketplaceListing>,
    pub checkouts: BTreeMap<String, MarketplaceCheckout>,
    pub installs: Vec<MarketplaceInstallReceipt>,
}

pub fn archive_marketplace_event(event: &MarketplaceEvent) -> Vec<u8> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(event)
        .expect("marketplace event must serialize through the rkyv wire boundary")
        .into_vec()
}

pub fn decode_marketplace_event(payload: &[u8]) -> Option<MarketplaceEvent> {
    edgerun_wire::from_bytes::<MarketplaceEvent, edgerun_wire::WireError>(payload).ok()
}

pub fn validate_fee_policy(policy: &MarketplaceFeePolicy) -> Result<(), MarketplaceError> {
    let total_bps = policy
        .edgerun_bps
        .checked_add(policy.app_bps)
        .ok_or(MarketplaceError::InvalidFeePolicy("basis points overflow"))?;

    if total_bps > policy.max_total_bps {
        return Err(MarketplaceError::InvalidFeePolicy(
            "fee split exceeds maximum total basis points",
        ));
    }
    if policy.edgerun_bps > 0 {
        validate_settlement_address(policy.edgerun_recipient.as_ref(), "edgerun fee recipient")?;
    }
    if policy.app_bps > 0 {
        validate_settlement_address(policy.app_recipient.as_ref(), "app fee recipient")?;
    }
    Ok(())
}

pub fn validate_listing(listing: &MarketplaceListing) -> Result<(), MarketplaceError> {
    require_non_empty(&listing.listing_id, "listing_id")?;
    require_non_empty(&listing.package_id, "package_id")?;
    require_non_empty(&listing.publisher_id, "publisher_id")?;
    require_non_empty(&listing.price_asset, "price_asset")?;
    require_non_empty(&listing.price_amount, "price_amount")?;
    require_non_empty(&listing.seller_settlement_asset, "seller_settlement_asset")?;
    require_non_empty(
        &listing.seller_settlement_address,
        "seller_settlement_address",
    )?;
    if !matches!(
        listing.status,
        value if value == ListingStatus::Draft as i32
            || value == ListingStatus::Active as i32
            || value == ListingStatus::Suspended as i32
            || value == ListingStatus::Expired as i32
    ) {
        return Err(MarketplaceError::InvalidStatus("unknown listing status"));
    }
    validate_fee_policy(&listing.fee_policy)
}

pub fn project_marketplace_events(events: &[MarketplaceEvent]) -> MarketplaceProjection {
    let mut projection = MarketplaceProjection::default();
    for event in events {
        apply_marketplace_event(&mut projection, event);
    }
    projection
}

pub fn apply_marketplace_event(projection: &mut MarketplaceProjection, event: &MarketplaceEvent) {
    match event {
        MarketplaceEvent::PublisherRegistered(publisher) => {
            projection
                .publishers
                .insert(publisher.publisher_id.clone(), publisher.clone());
        }
        MarketplaceEvent::PackagePublished(package) => {
            projection
                .packages
                .insert(package.package_id.clone(), package.clone());
        }
        MarketplaceEvent::ListingPublished(listing) => {
            projection
                .listings
                .insert(listing.listing_id.clone(), listing.clone());
        }
        MarketplaceEvent::ListingSuspended { listing_id, .. } => {
            if let Some(listing) = projection.listings.get_mut(listing_id) {
                listing.status = ListingStatus::Suspended as i32;
            }
        }
        MarketplaceEvent::CheckoutCreated(checkout) => {
            projection
                .checkouts
                .insert(checkout.checkout_id.clone(), checkout.clone());
        }
        MarketplaceEvent::CheckoutPaid {
            checkout_id,
            receipt_id,
            ..
        } => {
            if let Some(checkout) = projection.checkouts.get_mut(checkout_id) {
                checkout.status = CheckoutStatus::Paid as i32;
                checkout.receipt_id = receipt_id.clone();
            }
        }
        MarketplaceEvent::InstallAuthorized { .. } => {}
        MarketplaceEvent::PackageInstalled(receipt) => {
            projection.installs.push(receipt.clone());
        }
    }
}

pub fn authorize_install(
    projection: &MarketplaceProjection,
    checkout_id: &str,
    package_id: &str,
    install_command_id: Vec<u8>,
) -> Result<MarketplaceInstallReceipt, MarketplaceError> {
    let checkout = projection
        .checkouts
        .get(checkout_id)
        .ok_or(MarketplaceError::NotFound("checkout"))?;
    if checkout.status != CheckoutStatus::Paid as i32 || checkout.receipt_id.is_empty() {
        return Err(MarketplaceError::NotPaid);
    }

    let listing = projection
        .listings
        .get(&checkout.listing_id)
        .ok_or(MarketplaceError::NotFound("listing"))?;
    if listing.status != ListingStatus::Active as i32 {
        return Err(MarketplaceError::ListingNotActive);
    }
    if listing.package_id != package_id {
        return Err(MarketplaceError::PackageMismatch);
    }
    if !projection.packages.contains_key(package_id) {
        return Err(MarketplaceError::NotFound("package"));
    }

    Ok(MarketplaceInstallReceipt {
        listing_id: listing.listing_id.clone(),
        package_id: package_id.into(),
        receipt_id: checkout.receipt_id.clone(),
        install_command_id,
        installed_event_id: Vec::new(),
    })
}

fn validate_settlement_address(
    address: Option<&SettlementAddress>,
    field: &'static str,
) -> Result<(), MarketplaceError> {
    let address = address.ok_or(MarketplaceError::MissingField(field))?;
    require_non_empty(&address.asset_id, field)?;
    require_non_empty(&address.address, field)
}

fn require_non_empty(value: &str, field: &'static str) -> Result<(), MarketplaceError> {
    if value.trim().is_empty() {
        Err(MarketplaceError::MissingField(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn fee_policy() -> MarketplaceFeePolicy {
        MarketplaceFeePolicy {
            edgerun_bps: 300,
            app_bps: 100,
            max_total_bps: 500,
            edgerun_recipient: Some(SettlementAddress {
                asset_id: "USDT:tron".into(),
                address: "edgerun-fee-address".into(),
            }),
            app_recipient: Some(SettlementAddress {
                asset_id: "USDT:tron".into(),
                address: "app-fee-address".into(),
            }),
        }
    }

    fn package() -> MarketplacePackage {
        MarketplacePackage {
            package_id: "pkg-storefront".into(),
            app_package_object: ObjectRef {
                object_id: b"app-package-object".to_vec(),
                object_kind: None,
            },
            publisher_id: "pub-1".into(),
            version: "1.0.0".into(),
            content_hash: Digest {
                algorithm: 1,
                value: b"content-hash".to_vec(),
            },
            required_capabilities: Vec::new(),
            media_objects: Vec::new(),
        }
    }

    fn listing() -> MarketplaceListing {
        MarketplaceListing {
            listing_id: "listing-1".into(),
            package_id: "pkg-storefront".into(),
            publisher_id: "pub-1".into(),
            price_asset: "USDT:tron".into(),
            price_amount: "25.00".into(),
            seller_settlement_asset: "USDT:tron".into(),
            seller_settlement_address: "seller-address".into(),
            fee_policy: fee_policy(),
            status: ListingStatus::Active as i32,
            expires_at_ms: 0,
        }
    }

    fn checkout() -> MarketplaceCheckout {
        MarketplaceCheckout {
            checkout_id: "checkout-1".into(),
            listing_id: "listing-1".into(),
            buyer_node: None,
            install_target_node: None,
            payment_request_id: "pr-1".into(),
            quote_id: "quote-1".into(),
            order_id: "order-1".into(),
            receipt_id: String::new(),
            status: CheckoutStatus::Created as i32,
            fee_policy: fee_policy(),
        }
    }

    #[test]
    fn fee_policy_requires_recipients_for_nonzero_fees() {
        let mut policy = fee_policy();
        policy.edgerun_recipient = None;

        assert_eq!(
            validate_fee_policy(&policy),
            Err(MarketplaceError::MissingField("edgerun fee recipient"))
        );
    }

    #[test]
    fn listing_validation_rejects_fee_policy_over_limit() {
        let mut listing = listing();
        listing.fee_policy.app_bps = 300;

        assert_eq!(
            validate_listing(&listing),
            Err(MarketplaceError::InvalidFeePolicy(
                "fee split exceeds maximum total basis points"
            ))
        );
    }

    #[test]
    fn projection_marks_checkout_paid_from_committed_events() {
        let events = vec![
            MarketplaceEvent::PackagePublished(package()),
            MarketplaceEvent::ListingPublished(listing()),
            MarketplaceEvent::CheckoutCreated(checkout()),
            MarketplaceEvent::CheckoutPaid {
                checkout_id: "checkout-1".into(),
                receipt_id: "receipt-1".into(),
                paid_at_ms: 1_700_000_000_000,
            },
        ];

        let projection = project_marketplace_events(&events);
        let checkout = projection.checkouts.get("checkout-1").unwrap();

        assert_eq!(checkout.status, CheckoutStatus::Paid as i32);
        assert_eq!(checkout.receipt_id, "receipt-1");
    }

    #[test]
    fn install_requires_paid_checkout() {
        let projection = project_marketplace_events(&[
            MarketplaceEvent::PackagePublished(package()),
            MarketplaceEvent::ListingPublished(listing()),
            MarketplaceEvent::CheckoutCreated(checkout()),
        ]);

        assert_eq!(
            authorize_install(
                &projection,
                "checkout-1",
                "pkg-storefront",
                b"install-cmd".to_vec()
            ),
            Err(MarketplaceError::NotPaid)
        );
    }

    #[test]
    fn paid_checkout_authorizes_install_for_listing_package() {
        let projection = project_marketplace_events(&[
            MarketplaceEvent::PackagePublished(package()),
            MarketplaceEvent::ListingPublished(listing()),
            MarketplaceEvent::CheckoutCreated(checkout()),
            MarketplaceEvent::CheckoutPaid {
                checkout_id: "checkout-1".into(),
                receipt_id: "receipt-1".into(),
                paid_at_ms: 1_700_000_000_000,
            },
        ]);

        let receipt = authorize_install(
            &projection,
            "checkout-1",
            "pkg-storefront",
            b"install-cmd".to_vec(),
        )
        .expect("paid checkout should authorize install");

        assert_eq!(receipt.receipt_id, "receipt-1");
        assert_eq!(receipt.install_command_id, b"install-cmd".to_vec());
    }

    #[test]
    fn marketplace_event_roundtrips_through_rkyv_wire() {
        let event = MarketplaceEvent::ListingPublished(listing());

        let bytes = archive_marketplace_event(&event);
        let decoded = decode_marketplace_event(&bytes).expect("event should decode");

        assert_eq!(decoded, event);
    }
}
