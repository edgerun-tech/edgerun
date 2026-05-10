use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use edgerun_wire::{
    RELAY_WIRE_ABI_VERSION, RelayAck, RelayDeliveryReceipt, RelayDeliveryReport,
    RelayDeliveryReportReceipt, RelayDeliveryRequest, RelayIdentity, RelayMessage, RelaySubmit,
};

use crate::{
    MAX_PENDING_DELIVERIES, MAX_ROUTES, RelayWireError, report_hash, sha256_array,
    verify_delivery_receipt, verify_register, verify_report_receipt, verify_submit,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelayPeerKey {
    pub algorithm: u16,
    pub public_key: Vec<u8>,
}

impl From<&RelayIdentity> for RelayPeerKey {
    fn from(value: &RelayIdentity) -> Self {
        Self {
            algorithm: value.algorithm,
            public_key: value.public_key.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelayOutput<Peer> {
    Ack { peer: Peer, ack: RelayAck },
    Message { peer: Peer, message: RelayMessage },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelayEngineError {
    InvalidSignature,
    InvalidPayloadHash,
    PayloadTooLarge,
    RouteTableFull,
    PendingTableFull,
    NodeNotRegistered,
    UnknownMessage,
    WrongSigner,
    DuplicateMessage,
    Wire(RelayWireError),
}

impl RelayEngineError {
    pub fn code(&self) -> u16 {
        match self {
            Self::InvalidSignature => 401,
            Self::InvalidPayloadHash | Self::Wire(_) => 400,
            Self::PayloadTooLarge => 413,
            Self::RouteTableFull | Self::PendingTableFull => 503,
            Self::NodeNotRegistered | Self::UnknownMessage => 404,
            Self::WrongSigner | Self::DuplicateMessage => 409,
        }
    }

    pub fn text(&self) -> &'static str {
        match self {
            Self::InvalidSignature => "invalid signature",
            Self::InvalidPayloadHash => "invalid payload hash",
            Self::PayloadTooLarge => "relay payload is too large",
            Self::RouteTableFull => "relay route table is full",
            Self::PendingTableFull => "relay pending table is full",
            Self::NodeNotRegistered => "destination node is not registered",
            Self::UnknownMessage => "unknown relay message id",
            Self::WrongSigner => "receipt signer does not match pending delivery",
            Self::DuplicateMessage => "duplicate relay message id",
            Self::Wire(_) => "invalid relay wire packet",
        }
    }
}

#[derive(Clone)]
struct PendingDelivery<Peer> {
    sender: RelayIdentity,
    recipient: RelayIdentity,
    sender_peer: Peer,
    submit: RelaySubmit,
}

#[derive(Clone)]
pub struct RelayEngine<Peer>
where
    Peer: Clone + PartialEq,
{
    routes: BTreeMap<RelayPeerKey, Peer>,
    pending: BTreeMap<[u8; 32], PendingDelivery<Peer>>,
}

impl<Peer> Default for RelayEngine<Peer>
where
    Peer: Clone + PartialEq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Peer> RelayEngine<Peer>
where
    Peer: Clone + PartialEq,
{
    pub const fn new() -> Self {
        Self {
            routes: BTreeMap::new(),
            pending: BTreeMap::new(),
        }
    }

    pub fn registered_len(&self) -> usize {
        self.routes.len()
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn remove_peer(&mut self, peer: &Peer) {
        self.routes.retain(|_, route_peer| route_peer != peer);
        self.pending
            .retain(|_, pending| &pending.sender_peer != peer);
    }

    pub fn handle_message(&mut self, peer: Peer, message: RelayMessage) -> Vec<RelayOutput<Peer>> {
        let mut out = Vec::new();
        match self.handle_message_inner(peer.clone(), message, &mut out) {
            Ok(ack) => out.push(RelayOutput::Ack { peer, ack }),
            Err(error) => out.push(RelayOutput::Ack {
                peer,
                ack: RelayAck {
                    ok: false,
                    code: error.code(),
                    text: String::from(error.text()),
                },
            }),
        }
        out
    }

    fn handle_message_inner(
        &mut self,
        peer: Peer,
        message: RelayMessage,
        out: &mut Vec<RelayOutput<Peer>>,
    ) -> Result<RelayAck, RelayEngineError> {
        match message {
            RelayMessage::Register(register) => {
                if !verify_register(&register) {
                    return Err(RelayEngineError::InvalidSignature);
                }
                let key = RelayPeerKey::from(&register.node);
                if self.routes.len() >= MAX_ROUTES && !self.routes.contains_key(&key) {
                    return Err(RelayEngineError::RouteTableFull);
                }
                self.routes.insert(key, peer);
                Ok(ok_ack(200, "registered"))
            }
            RelayMessage::Submit(submit) => {
                self.accept_submit(peer, submit, out)?;
                Ok(ok_ack(202, "delivery requested"))
            }
            RelayMessage::DeliveryReceipt(receipt) => {
                self.accept_delivery_receipt(receipt, out)?;
                Ok(ok_ack(202, "delivery report sent"))
            }
            RelayMessage::DeliveryReportReceipt(receipt) => {
                self.accept_report_receipt(receipt)?;
                Ok(ok_ack(200, "delivery report acknowledged"))
            }
            RelayMessage::DeliveryRequest(_)
            | RelayMessage::DeliveryReport(_)
            | RelayMessage::Ack(_) => Err(RelayEngineError::Wire(RelayWireError::InvalidPacket)),
        }
    }

    fn accept_submit(
        &mut self,
        sender_peer: Peer,
        submit: RelaySubmit,
        out: &mut Vec<RelayOutput<Peer>>,
    ) -> Result<(), RelayEngineError> {
        if submit.payload.len() > crate::MAX_PAYLOAD_LEN {
            return Err(RelayEngineError::PayloadTooLarge);
        }
        if sha256_array(&submit.payload) != submit.payload_sha256 {
            return Err(RelayEngineError::InvalidPayloadHash);
        }
        if !verify_submit(&submit) {
            return Err(RelayEngineError::InvalidSignature);
        }
        if self.pending.contains_key(&submit.message_id) {
            return Err(RelayEngineError::DuplicateMessage);
        }
        if self.pending.len() >= MAX_PENDING_DELIVERIES {
            return Err(RelayEngineError::PendingTableFull);
        }
        let recipient_peer = self
            .routes
            .get(&RelayPeerKey::from(&submit.to))
            .cloned()
            .ok_or(RelayEngineError::NodeNotRegistered)?;
        self.pending.insert(
            submit.message_id,
            PendingDelivery {
                sender: submit.from.clone(),
                recipient: submit.to.clone(),
                sender_peer,
                submit: submit.clone(),
            },
        );
        out.push(RelayOutput::Message {
            peer: recipient_peer,
            message: RelayMessage::DeliveryRequest(RelayDeliveryRequest {
                abi_version: RELAY_WIRE_ABI_VERSION,
                flags: 1,
                relay_id: b"edgerun-relay".to_vec(),
                submit,
                received_unix_ms: 0,
            }),
        });
        Ok(())
    }

    fn accept_delivery_receipt(
        &mut self,
        receipt: RelayDeliveryReceipt,
        out: &mut Vec<RelayOutput<Peer>>,
    ) -> Result<(), RelayEngineError> {
        let pending = self
            .pending
            .get(&receipt.message_id)
            .cloned()
            .ok_or(RelayEngineError::UnknownMessage)?;
        if pending.recipient != receipt.recipient {
            return Err(RelayEngineError::WrongSigner);
        }
        if !verify_delivery_receipt(&receipt) {
            return Err(RelayEngineError::InvalidSignature);
        }
        out.push(RelayOutput::Message {
            peer: pending.sender_peer,
            message: RelayMessage::DeliveryReport(RelayDeliveryReport {
                abi_version: RELAY_WIRE_ABI_VERSION,
                flags: 1,
                relay_id: b"edgerun-relay".to_vec(),
                submit: pending.submit,
                recipient_receipt: receipt,
                reported_unix_ms: 0,
            }),
        });
        Ok(())
    }

    fn accept_report_receipt(
        &mut self,
        receipt: RelayDeliveryReportReceipt,
    ) -> Result<(), RelayEngineError> {
        let pending = self
            .pending
            .get(&receipt.message_id)
            .cloned()
            .ok_or(RelayEngineError::UnknownMessage)?;
        if pending.sender != receipt.sender {
            return Err(RelayEngineError::WrongSigner);
        }
        if !verify_report_receipt(&receipt) {
            return Err(RelayEngineError::InvalidSignature);
        }
        self.pending.remove(&receipt.message_id);
        Ok(())
    }
}

fn ok_ack(code: u16, text: &'static str) -> RelayAck {
    RelayAck {
        ok: true,
        code,
        text: String::from(text),
    }
}
