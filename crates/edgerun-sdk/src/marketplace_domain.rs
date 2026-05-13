//! EdgeRun marketplace domain model.
//!
//! This module does not own stream authority, settlement custody, or HTTP
//! ingress. It defines rkyv-native marketplace facts that SDK callers can
//! commit through the node stream and project back into browse, checkout, and
//! install state.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use edgerun_protocols::core_protocol::protocol::edgerun_wallet_v0::{PaymentRequest, Receipt};
use edgerun_protocols::core_protocol::protocol::{
    AppIntent, CapabilityDescriptor, CommandEnvelope, Digest, IdentityRef, NodeRef, ObjectRef,
    Timestamp, command_envelope,
};
use edgerun_protocols::wire as edgerun_wire;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarketplaceError {
    MissingField(&'static str),
    InvalidCommissionPolicy(&'static str),
    InvalidStatus(&'static str),
    NotFound(&'static str),
    NotPaid,
    ListingNotActive,
    PackageMismatch,
    ReceiptMismatch(&'static str),
}

impl core::fmt::Display for MarketplaceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "missing marketplace field: {field}"),
            Self::InvalidCommissionPolicy(reason) => {
                write!(f, "invalid marketplace commission policy: {reason}")
            }
            Self::InvalidStatus(reason) => write!(f, "invalid marketplace status: {reason}"),
            Self::NotFound(entity) => write!(f, "marketplace entity not found: {entity}"),
            Self::NotPaid => f.write_str("marketplace checkout is not paid"),
            Self::ListingNotActive => f.write_str("marketplace listing is not active"),
            Self::PackageMismatch => f.write_str("marketplace package mismatch"),
            Self::ReceiptMismatch(reason) => write!(f, "marketplace receipt mismatch: {reason}"),
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
pub struct MarketplaceCommissionPolicy {
    pub edgerun_bps: u32,
    pub app_bps: u32,
    pub affiliate_bps: u32,
    pub max_total_bps: u32,
    pub edgerun_recipient: Option<SettlementAddress>,
    pub app_recipient: Option<SettlementAddress>,
    pub affiliate_recipient: Option<SettlementAddress>,
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
pub struct MarketplacePayoutSplit {
    pub asset_id: String,
    pub gross_minor_units: u128,
    pub seller_minor_units: u128,
    pub edgerun_commission_minor_units: u128,
    pub app_commission_minor_units: u128,
    pub affiliate_commission_minor_units: u128,
    pub seller_recipient: SettlementAddress,
    pub edgerun_recipient: Option<SettlementAddress>,
    pub app_recipient: Option<SettlementAddress>,
    pub affiliate_recipient: Option<SettlementAddress>,
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
    pub commission_policy: MarketplaceCommissionPolicy,
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
    pub commission_policy: MarketplaceCommissionPolicy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarketplaceSettlementCommandDraft {
    pub command_id: Vec<u8>,
    pub target_node: NodeRef,
    pub issuer: Option<IdentityRef>,
    pub command_type: i32,
    pub issued_at: Option<Timestamp>,
    pub expires_at: Option<Timestamp>,
    pub idempotency_key: Vec<u8>,
    pub app_id: Vec<u8>,
    pub app_signature: Vec<u8>,
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

pub fn validate_commission_policy(
    policy: &MarketplaceCommissionPolicy,
) -> Result<(), MarketplaceError> {
    let total_bps = policy
        .edgerun_bps
        .checked_add(policy.app_bps)
        .and_then(|value| value.checked_add(policy.affiliate_bps))
        .ok_or(MarketplaceError::InvalidCommissionPolicy(
            "basis points overflow",
        ))?;

    if total_bps > policy.max_total_bps {
        return Err(MarketplaceError::InvalidCommissionPolicy(
            "commission split exceeds maximum total basis points",
        ));
    }
    if policy.edgerun_bps > 0 {
        validate_settlement_address(
            policy.edgerun_recipient.as_ref(),
            "edgerun commission recipient",
        )?;
    }
    if policy.app_bps > 0 {
        validate_settlement_address(policy.app_recipient.as_ref(), "app commission recipient")?;
    }
    if policy.affiliate_bps > 0 {
        validate_settlement_address(
            policy.affiliate_recipient.as_ref(),
            "affiliate commission recipient",
        )?;
    }
    Ok(())
}

pub fn calculate_payout_split_minor_units(
    listing: &MarketplaceListing,
    gross_minor_units: u128,
) -> Result<MarketplacePayoutSplit, MarketplaceError> {
    validate_listing(listing)?;
    let edgerun_commission_minor_units =
        bps_amount(gross_minor_units, listing.commission_policy.edgerun_bps)?;
    let app_commission_minor_units =
        bps_amount(gross_minor_units, listing.commission_policy.app_bps)?;
    let affiliate_commission_minor_units =
        bps_amount(gross_minor_units, listing.commission_policy.affiliate_bps)?;
    let total_commission_minor_units = edgerun_commission_minor_units
        .checked_add(app_commission_minor_units)
        .and_then(|value| value.checked_add(affiliate_commission_minor_units))
        .ok_or(MarketplaceError::InvalidCommissionPolicy(
            "commission amount overflow",
        ))?;
    let seller_minor_units = gross_minor_units
        .checked_sub(total_commission_minor_units)
        .ok_or(MarketplaceError::InvalidCommissionPolicy(
            "commission amount exceeds gross amount",
        ))?;

    Ok(MarketplacePayoutSplit {
        asset_id: listing.seller_settlement_asset.clone(),
        gross_minor_units,
        seller_minor_units,
        edgerun_commission_minor_units,
        app_commission_minor_units,
        affiliate_commission_minor_units,
        seller_recipient: SettlementAddress {
            asset_id: listing.seller_settlement_asset.clone(),
            address: listing.seller_settlement_address.clone(),
        },
        edgerun_recipient: listing.commission_policy.edgerun_recipient.clone(),
        app_recipient: listing.commission_policy.app_recipient.clone(),
        affiliate_recipient: listing.commission_policy.affiliate_recipient.clone(),
    })
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
    validate_commission_policy(&listing.commission_policy)
}

pub fn build_payment_request_for_listing(
    listing: &MarketplaceListing,
    request_id: impl Into<String>,
    created_at_ms: u64,
    expires_at_ms: u64,
    pay_asset_id: Option<String>,
) -> Result<PaymentRequest, MarketplaceError> {
    validate_listing(listing)?;
    if listing.status != ListingStatus::Active as i32 {
        return Err(MarketplaceError::ListingNotActive);
    }

    Ok(PaymentRequest {
        request_id: request_id.into(),
        settlement_asset_id: listing.seller_settlement_asset.clone(),
        settlement_amount: listing.price_amount.clone(),
        recipient_address: Some(listing.seller_settlement_address.clone()),
        description: Some(format!("marketplace listing {}", listing.listing_id)),
        created_at_ms,
        expires_at_ms,
        pay_asset_id,
    })
}

pub fn create_checkout_for_listing(
    listing: &MarketplaceListing,
    checkout_id: impl Into<String>,
    payment_request_id: impl Into<String>,
    buyer_node: Option<NodeRef>,
    install_target_node: Option<NodeRef>,
) -> Result<MarketplaceCheckout, MarketplaceError> {
    validate_listing(listing)?;
    if listing.status != ListingStatus::Active as i32 {
        return Err(MarketplaceError::ListingNotActive);
    }

    Ok(MarketplaceCheckout {
        checkout_id: checkout_id.into(),
        listing_id: listing.listing_id.clone(),
        buyer_node,
        install_target_node,
        payment_request_id: payment_request_id.into(),
        quote_id: String::new(),
        order_id: String::new(),
        receipt_id: String::new(),
        status: CheckoutStatus::Created as i32,
        commission_policy: listing.commission_policy.clone(),
    })
}

pub fn build_settlement_command_for_payment_request(
    payment_request: &PaymentRequest,
    draft: MarketplaceSettlementCommandDraft,
) -> CommandEnvelope {
    let payload_bytes = archive_payment_request_intent(payment_request);
    let app_intent = build_app_intent(draft.app_id, payload_bytes.clone(), draft.app_signature);

    CommandEnvelope {
        envelope_version: 1,
        command_id: draft.command_id,
        target_node: Some(draft.target_node),
        issuer: draft.issuer,
        command_type: draft.command_type,
        command_version: 1,
        issued_at: draft.issued_at,
        not_before: None,
        expires_at: draft.expires_at,
        idempotency_key: draft.idempotency_key,
        delegation_chain: Vec::new(),
        requested_assurance: None,
        command_metadata: None,
        signatures: Vec::new(),
        app_intent: encode_app_intent(&app_intent),
        payload: Some(command_envelope::Payload::InlinePayload(payload_bytes)),
    }
}

pub fn archive_payment_request_intent(request: &PaymentRequest) -> Vec<u8> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(request)
        .expect("payment request must serialize through the rkyv wire boundary")
        .into_vec()
}

pub fn build_app_intent(app_id: Vec<u8>, payload: Vec<u8>, signature: Vec<u8>) -> AppIntent {
    AppIntent {
        app_id,
        payload,
        signature,
    }
}

pub fn encode_app_intent(intent: &AppIntent) -> Vec<u8> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(intent)
        .expect("app intent must serialize through the rkyv wire boundary")
        .into_vec()
}

pub fn checkout_paid_event_from_receipt(
    checkout: &MarketplaceCheckout,
    listing: &MarketplaceListing,
    receipt: &Receipt,
    paid_at_ms: u64,
) -> Result<MarketplaceEvent, MarketplaceError> {
    if checkout.listing_id != listing.listing_id {
        return Err(MarketplaceError::ReceiptMismatch(
            "checkout does not reference listing",
        ));
    }
    if checkout.commission_policy != listing.commission_policy {
        return Err(MarketplaceError::ReceiptMismatch(
            "checkout commission policy differs from listing",
        ));
    }
    if receipt.receipt_id.is_empty() {
        return Err(MarketplaceError::MissingField("receipt_id"));
    }
    if !checkout.order_id.is_empty() && receipt.order_id != checkout.order_id {
        return Err(MarketplaceError::ReceiptMismatch(
            "receipt order does not match checkout",
        ));
    }
    if receipt.settlement_asset_id != listing.seller_settlement_asset {
        return Err(MarketplaceError::ReceiptMismatch(
            "receipt settlement asset does not match listing",
        ));
    }
    if receipt.settlement_amount != listing.price_amount {
        return Err(MarketplaceError::ReceiptMismatch(
            "receipt settlement amount does not match listing",
        ));
    }

    Ok(MarketplaceEvent::CheckoutPaid {
        checkout_id: checkout.checkout_id.clone(),
        receipt_id: receipt.receipt_id.clone(),
        paid_at_ms,
    })
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

fn bps_amount(gross_minor_units: u128, bps: u32) -> Result<u128, MarketplaceError> {
    gross_minor_units
        .checked_mul(u128::from(bps))
        .and_then(|value| value.checked_div(10_000))
        .ok_or(MarketplaceError::InvalidCommissionPolicy(
            "commission calculation overflow",
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use edgerun_protocols::core_protocol::protocol::command_envelope;
    use edgerun_protocols::core_protocol::protocol::{AppIntent, CommandType};

    fn commission_policy() -> MarketplaceCommissionPolicy {
        MarketplaceCommissionPolicy {
            edgerun_bps: 300,
            app_bps: 100,
            affiliate_bps: 50,
            max_total_bps: 500,
            edgerun_recipient: Some(SettlementAddress {
                asset_id: "USDT:tron".into(),
                address: "edgerun-commission-address".into(),
            }),
            app_recipient: Some(SettlementAddress {
                asset_id: "USDT:tron".into(),
                address: "app-commission-address".into(),
            }),
            affiliate_recipient: Some(SettlementAddress {
                asset_id: "USDT:tron".into(),
                address: "affiliate-commission-address".into(),
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
            commission_policy: commission_policy(),
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
            commission_policy: commission_policy(),
        }
    }

    fn receipt() -> Receipt {
        Receipt {
            receipt_id: "receipt-1".into(),
            order_id: "order-1".into(),
            settlement_asset_id: "USDT:tron".into(),
            pay_asset_id: "BTC:bitcoin".into(),
            settlement_amount: "25.00".into(),
            pay_amount: "0.00025".into(),
            deposit_tx: None,
            payout_tx: None,
            completed_at_ms: 1_700_000_100_000,
            receipt_hash: b"receipt-hash".to_vec(),
            receipt_signature: None,
        }
    }

    #[test]
    fn commission_policy_requires_recipients_for_nonzero_commissions() {
        let mut policy = commission_policy();
        policy.edgerun_recipient = None;

        assert_eq!(
            validate_commission_policy(&policy),
            Err(MarketplaceError::MissingField(
                "edgerun commission recipient"
            ))
        );
    }

    #[test]
    fn listing_validation_rejects_commission_policy_over_limit() {
        let mut listing = listing();
        listing.commission_policy.app_bps = 300;

        assert_eq!(
            validate_listing(&listing),
            Err(MarketplaceError::InvalidCommissionPolicy(
                "commission split exceeds maximum total basis points"
            ))
        );
    }

    #[test]
    fn payout_split_calculates_non_custodial_commission_outputs() {
        let split = calculate_payout_split_minor_units(&listing(), 2_500_000)
            .expect("valid listing should calculate split");

        assert_eq!(split.asset_id, "USDT:tron");
        assert_eq!(split.gross_minor_units, 2_500_000);
        assert_eq!(split.edgerun_commission_minor_units, 75_000);
        assert_eq!(split.app_commission_minor_units, 25_000);
        assert_eq!(split.affiliate_commission_minor_units, 12_500);
        assert_eq!(split.seller_minor_units, 2_387_500);
        assert_eq!(split.seller_recipient.address, "seller-address");
        assert_eq!(
            split
                .edgerun_recipient
                .as_ref()
                .map(|value| value.address.as_str()),
            Some("edgerun-commission-address")
        );
        assert_eq!(
            split
                .app_recipient
                .as_ref()
                .map(|value| value.address.as_str()),
            Some("app-commission-address")
        );
        assert_eq!(
            split
                .affiliate_recipient
                .as_ref()
                .map(|value| value.address.as_str()),
            Some("affiliate-commission-address")
        );
    }

    #[test]
    fn payment_request_from_listing_targets_seller_settlement() {
        let request = build_payment_request_for_listing(
            &listing(),
            "pr-1",
            1_700_000_000_000,
            1_700_000_900_000,
            Some("BTC:bitcoin".into()),
        )
        .expect("active listing should produce payment request");

        assert_eq!(request.request_id, "pr-1");
        assert_eq!(request.settlement_asset_id, "USDT:tron");
        assert_eq!(request.settlement_amount, "25.00");
        assert_eq!(request.recipient_address.as_deref(), Some("seller-address"));
        assert_eq!(request.pay_asset_id.as_deref(), Some("BTC:bitcoin"));
    }

    #[test]
    fn settlement_command_carries_listing_payment_request_intent() {
        let request = build_payment_request_for_listing(
            &listing(),
            "pr-1",
            1_700_000_000_000,
            1_700_000_900_000,
            None,
        )
        .expect("payment request");

        let command = build_settlement_command_for_payment_request(
            &request,
            MarketplaceSettlementCommandDraft {
                command_id: b"cmd-checkout-1".to_vec(),
                target_node: NodeRef {
                    node_id: b"settlement-node".to_vec(),
                },
                issuer: None,
                command_type: CommandType::StoreAndForward as i32,
                issued_at: None,
                expires_at: None,
                idempotency_key: b"checkout-1".to_vec(),
                app_id: b"marketplace-app".to_vec(),
                app_signature: b"signature-placeholder".to_vec(),
            },
        );

        assert_eq!(
            command.target_node.unwrap().node_id,
            b"settlement-node".to_vec()
        );

        let intent =
            edgerun_wire::from_bytes::<AppIntent, edgerun_wire::WireError>(&command.app_intent)
                .expect("app intent should decode");
        assert_eq!(intent.app_id, b"marketplace-app".to_vec());

        match command.payload {
            Some(command_envelope::Payload::InlinePayload(bytes)) => {
                let decoded =
                    edgerun_wire::from_bytes::<PaymentRequest, edgerun_wire::WireError>(&bytes)
                        .expect("payment request should decode");
                assert_eq!(decoded, request);
                assert_eq!(intent.payload, bytes);
            }
            _ => panic!("settlement command should carry inline payment request"),
        }
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
    fn receipt_matching_listing_creates_paid_event() {
        let mut checkout = checkout();
        checkout.order_id = "order-1".into();

        let event =
            checkout_paid_event_from_receipt(&checkout, &listing(), &receipt(), 1_700_000_100_000)
                .expect("receipt should match checkout");

        assert_eq!(
            event,
            MarketplaceEvent::CheckoutPaid {
                checkout_id: "checkout-1".into(),
                receipt_id: "receipt-1".into(),
                paid_at_ms: 1_700_000_100_000,
            }
        );
    }

    #[test]
    fn receipt_amount_mismatch_is_rejected() {
        let mut checkout = checkout();
        checkout.order_id = "order-1".into();
        let mut receipt = receipt();
        receipt.settlement_amount = "24.99".into();

        assert_eq!(
            checkout_paid_event_from_receipt(&checkout, &listing(), &receipt, 1),
            Err(MarketplaceError::ReceiptMismatch(
                "receipt settlement amount does not match listing"
            ))
        );
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
