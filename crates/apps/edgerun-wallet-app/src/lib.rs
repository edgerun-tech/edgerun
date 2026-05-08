#![no_std]

//! EdgeRun wallet app.
//!
//! This app has no public HTTP route. It is intended to be installed into an
//! `edgerun-node` runtime and addressed through `RuntimeAppMessage` payloads.
//! The payloads are rkyv records; HTTP/JSON adapters can be separate edge apps.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use edgerun_protocols::core_protocol::protocol::edgerun_wallet_v0::{
    Asset, PaymentRequest, Receipt, TxRef,
};
use edgerun_sdk::runtime_api::{self, RuntimeAppInstall};
use edgerun_wallet::DecimalAmount;
use edgerun_wire::{
    from_bytes, sdk_wire_bytes, to_bytes, Archive, Deserialize, RuntimeAppMessage, SdkWireRecord,
    Serialize, WireError, SDK_WIRE_ABI_VERSION,
};

pub const APP_SLUG: &[u8] = b"edgerun-wallet-app";
pub const APP_VERSION: &[u8] = b"0.1.0";
pub const STORAGE_NAMESPACE: &[u8] = b"edgerun-wallet-app/state";

pub const WALLET_APP_MESSAGE_KIND_REQUEST: u16 = 0x7101;
pub const WALLET_APP_MESSAGE_KIND_RESPONSE: u16 = 0x7102;

#[derive(Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct WalletAppRequest {
    pub abi_version: u16,
    pub flags: u32,
    pub request_id: [u8; 32],
    pub command: WalletAppCommand,
}

#[derive(Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub enum WalletAppCommand {
    ListAssets,
    UpsertAsset(Asset),
    ListBalances,
    UpsertBalance(WalletBalance),
    CreatePaymentRequest(PaymentRequestDraft),
    GetPaymentRequest { request_id: String },
    ListPaymentRequests,
    RecordReceipt(Receipt),
    GetReceipt { receipt_id: String },
}

#[derive(Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct PaymentRequestDraft {
    pub settlement_asset_id: String,
    pub settlement_amount: String,
    pub recipient_address: Option<String>,
    pub description: Option<String>,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub pay_asset_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct WalletBalance {
    pub asset_id: String,
    pub confirmed_amount: String,
    pub pending_amount: String,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct WalletAppResponse {
    pub abi_version: u16,
    pub flags: u32,
    pub request_id: [u8; 32],
    pub result: WalletAppResult,
}

#[derive(Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub enum WalletAppResult {
    Assets(Vec<Asset>),
    Balances(Vec<WalletBalance>),
    PaymentRequest(PaymentRequest),
    PaymentRequests(Vec<PaymentRequest>),
    Receipt(Receipt),
    NotFound,
    Accepted,
    Rejected { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalletAppError {
    InvalidWireRecord,
    InvalidAbi,
    InvalidMessageKind,
    WrongDestination,
}

#[derive(Clone, Debug, Default)]
pub struct WalletAppState {
    assets: BTreeMap<String, Asset>,
    balances: BTreeMap<String, WalletBalance>,
    payment_requests: BTreeMap<String, PaymentRequest>,
    receipts: BTreeMap<String, Receipt>,
    next_payment_request_seq: u64,
}

impl WalletAppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_request(&mut self, request: WalletAppRequest) -> WalletAppResponse {
        let result = match request.command {
            WalletAppCommand::ListAssets => {
                WalletAppResult::Assets(self.assets.values().cloned().collect())
            }
            WalletAppCommand::UpsertAsset(asset) => {
                self.assets.insert(asset.id.clone(), asset);
                WalletAppResult::Accepted
            }
            WalletAppCommand::ListBalances => {
                WalletAppResult::Balances(self.balances.values().cloned().collect())
            }
            WalletAppCommand::UpsertBalance(balance) => match validate_balance(&balance) {
                Ok(()) => {
                    self.balances.insert(balance.asset_id.clone(), balance);
                    WalletAppResult::Accepted
                }
                Err(reason) => WalletAppResult::Rejected { reason },
            },
            WalletAppCommand::CreatePaymentRequest(draft) => self.create_payment_request(draft),
            WalletAppCommand::GetPaymentRequest { request_id } => self
                .payment_requests
                .get(&request_id)
                .cloned()
                .map(WalletAppResult::PaymentRequest)
                .unwrap_or(WalletAppResult::NotFound),
            WalletAppCommand::ListPaymentRequests => {
                WalletAppResult::PaymentRequests(self.payment_requests.values().cloned().collect())
            }
            WalletAppCommand::RecordReceipt(receipt) => {
                self.receipts.insert(receipt.receipt_id.clone(), receipt);
                WalletAppResult::Accepted
            }
            WalletAppCommand::GetReceipt { receipt_id } => self
                .receipts
                .get(&receipt_id)
                .cloned()
                .map(WalletAppResult::Receipt)
                .unwrap_or(WalletAppResult::NotFound),
        };
        WalletAppResponse {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            request_id: request.request_id,
            result,
        }
    }

    fn create_payment_request(&mut self, draft: PaymentRequestDraft) -> WalletAppResult {
        if DecimalAmount::parse(&draft.settlement_amount).is_none() {
            return WalletAppResult::Rejected {
                reason: "invalid settlement amount".to_string(),
            };
        }
        if draft.expires_at_ms <= draft.created_at_ms {
            return WalletAppResult::Rejected {
                reason: "expires_at_ms must be after created_at_ms".to_string(),
            };
        }
        self.next_payment_request_seq += 1;
        let request = PaymentRequest {
            request_id: alloc::format!("wpr_{}", self.next_payment_request_seq),
            settlement_asset_id: draft.settlement_asset_id,
            settlement_amount: draft.settlement_amount,
            recipient_address: draft.recipient_address,
            description: draft.description,
            created_at_ms: draft.created_at_ms,
            expires_at_ms: draft.expires_at_ms,
            pay_asset_id: draft.pay_asset_id,
        };
        self.payment_requests
            .insert(request.request_id.clone(), request.clone());
        WalletAppResult::PaymentRequest(request)
    }
}

pub struct WalletAppRuntime {
    app_id: [u8; 32],
    state: WalletAppState,
}

impl WalletAppRuntime {
    pub fn new(app_id: [u8; 32]) -> Self {
        Self {
            app_id,
            state: WalletAppState::new(),
        }
    }

    pub fn state(&self) -> &WalletAppState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut WalletAppState {
        &mut self.state
    }

    pub fn handle_message(
        &mut self,
        message: RuntimeAppMessage,
    ) -> Result<RuntimeAppMessage, WalletAppError> {
        if message.to_app_id != self.app_id {
            return Err(WalletAppError::WrongDestination);
        }
        if message.message_kind != WALLET_APP_MESSAGE_KIND_REQUEST {
            return Err(WalletAppError::InvalidMessageKind);
        }
        let request = decode_wallet_app_request(&message.payload)?;
        let response_payload = encode_wallet_app_response(&self.state.handle_request(request));
        Ok(runtime_app_message(
            self.app_id,
            message.from_app_id,
            WALLET_APP_MESSAGE_KIND_RESPONSE,
            response_payload,
        ))
    }
}

pub fn runtime_app_install(developer_id: [u8; 32]) -> RuntimeAppInstall {
    let app_id = runtime_api::app_id(APP_SLUG, &developer_id);
    let manifest_sha256 = runtime_api::manifest_sha256(APP_SLUG, APP_VERSION, &[]);
    let release_id = runtime_api::release_id(&app_id, APP_VERSION, &manifest_sha256);
    runtime_api::app_install(
        app_id,
        release_id,
        edgerun_sdk::sha256(APP_SLUG),
        developer_id,
        manifest_sha256,
        Vec::new(),
        alloc::vec![STORAGE_NAMESPACE.to_vec()],
    )
}

pub fn encode_wallet_app_request(request: &WalletAppRequest) -> Vec<u8> {
    to_bytes::<WireError>(request)
        .expect("wallet app request must serialize through rkyv")
        .into_vec()
}

pub fn decode_wallet_app_request(bytes: &[u8]) -> Result<WalletAppRequest, WalletAppError> {
    let owned = bytes.to_vec();
    let request = from_bytes::<WalletAppRequest, WireError>(&owned)
        .map_err(|_| WalletAppError::InvalidWireRecord)?;
    if request.abi_version != SDK_WIRE_ABI_VERSION || request.flags & 1 != 1 {
        return Err(WalletAppError::InvalidAbi);
    }
    Ok(request)
}

pub fn encode_wallet_app_response(response: &WalletAppResponse) -> Vec<u8> {
    to_bytes::<WireError>(response)
        .expect("wallet app response must serialize through rkyv")
        .into_vec()
}

pub fn decode_wallet_app_response(bytes: &[u8]) -> Result<WalletAppResponse, WalletAppError> {
    let owned = bytes.to_vec();
    let response = from_bytes::<WalletAppResponse, WireError>(&owned)
        .map_err(|_| WalletAppError::InvalidWireRecord)?;
    if response.abi_version != SDK_WIRE_ABI_VERSION || response.flags & 1 != 1 {
        return Err(WalletAppError::InvalidAbi);
    }
    Ok(response)
}

pub fn runtime_app_message(
    from_app_id: [u8; 32],
    to_app_id: [u8; 32],
    message_kind: u16,
    payload: Vec<u8>,
) -> RuntimeAppMessage {
    RuntimeAppMessage {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        from_app_id,
        to_app_id,
        message_kind,
        payload_sha256: edgerun_sdk::sha256(&payload),
        payload,
    }
}

pub fn runtime_app_message_bytes(message: RuntimeAppMessage) -> Vec<u8> {
    sdk_wire_bytes(&SdkWireRecord::RuntimeAppMessage(message))
}

fn validate_balance(balance: &WalletBalance) -> Result<(), String> {
    if DecimalAmount::parse(&balance.confirmed_amount).is_none() {
        return Err("invalid confirmed amount".to_string());
    }
    if DecimalAmount::parse(&balance.pending_amount).is_none() {
        return Err("invalid pending amount".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn btc() -> Asset {
        Asset {
            id: "BTC:bitcoin".to_string(),
            symbol: "BTC".to_string(),
            network: "bitcoin".to_string(),
            contract: None,
            decimals: 8,
            name: Some("Bitcoin".to_string()),
        }
    }

    #[test]
    fn wallet_app_roundtrips_internal_rkyv_request_and_response() {
        let request = WalletAppRequest {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            request_id: [7; 32],
            command: WalletAppCommand::UpsertAsset(btc()),
        };
        let decoded =
            decode_wallet_app_request(&encode_wallet_app_request(&request)).expect("decode");
        assert_eq!(decoded, request);

        let mut state = WalletAppState::new();
        let response = state.handle_request(decoded);
        assert_eq!(response.result, WalletAppResult::Accepted);
        assert_eq!(state.assets.len(), 1);
    }

    #[test]
    fn wallet_app_handles_runtime_message_without_http() {
        let wallet_app_id = [1; 32];
        let dashboard_app_id = [2; 32];
        let request = WalletAppRequest {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            request_id: [9; 32],
            command: WalletAppCommand::CreatePaymentRequest(PaymentRequestDraft {
                settlement_asset_id: "BTC:bitcoin".to_string(),
                settlement_amount: "0.01".to_string(),
                recipient_address: Some("bc1seller".to_string()),
                description: Some("invoice".to_string()),
                created_at_ms: 100,
                expires_at_ms: 200,
                pay_asset_id: None,
            }),
        };
        let message = runtime_app_message(
            dashboard_app_id,
            wallet_app_id,
            WALLET_APP_MESSAGE_KIND_REQUEST,
            encode_wallet_app_request(&request),
        );
        let mut runtime = WalletAppRuntime::new(wallet_app_id);
        let response_message = runtime.handle_message(message).expect("handle message");
        assert_eq!(response_message.from_app_id, wallet_app_id);
        assert_eq!(response_message.to_app_id, dashboard_app_id);
        assert_eq!(
            response_message.message_kind,
            WALLET_APP_MESSAGE_KIND_RESPONSE
        );
        let response =
            decode_wallet_app_response(&response_message.payload).expect("decode response");
        assert!(matches!(
            response.result,
            WalletAppResult::PaymentRequest(_)
        ));
    }

    #[test]
    fn wallet_app_rejects_invalid_money_amounts() {
        let mut state = WalletAppState::new();
        let response = state.handle_request(WalletAppRequest {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            request_id: [3; 32],
            command: WalletAppCommand::UpsertBalance(WalletBalance {
                asset_id: "BTC:bitcoin".to_string(),
                confirmed_amount: "-1".to_string(),
                pending_amount: "0".to_string(),
                updated_at_ms: 1,
            }),
        });
        assert!(matches!(response.result, WalletAppResult::Rejected { .. }));
    }
}
