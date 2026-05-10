#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
mod std_runtime;

#[cfg(feature = "std")]
pub use std_runtime::*;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(not(feature = "std"))]
use core::fmt;

#[cfg(not(feature = "std"))]
pub mod nostd_relay;
#[cfg(all(not(feature = "std"), feature = "virtio"))]
pub mod nostd_virtio;

#[cfg(not(feature = "std"))]
use edgerun_crypto::sha256;
#[cfg(all(not(feature = "std"), feature = "p256-message"))]
use edgerun_protocols::verify::verify_message_signature;
#[cfg(not(feature = "std"))]
use edgerun_wire::{
    RELAY_DELIVERY_STATUS_ACCEPTED, RELAY_REPORT_STATUS_ACCEPTED, RELAY_WIRE_ABI_VERSION,
    RelayDeliveryReceipt, RelayDeliveryReport, RelayDeliveryReportReceipt, RelayDeliveryRequest,
    RelayIdentity, RelayMessage, RelayRegister, RelaySignature, RelaySubmit,
    SIGNATURE_ALGORITHM_ECDSA_P256_SHA256, SIGNATURE_ALGORITHM_ED25519, relay_message_bytes,
    relay_message_from_bytes,
};

#[cfg(not(feature = "std"))]
pub const MAX_FRAME_LEN: usize = 1024 * 1024;
#[cfg(not(feature = "std"))]
pub const MAX_PAYLOAD_LEN: usize = 512 * 1024;
#[cfg(not(feature = "std"))]
pub const MAX_ROUTES: usize = 65_536;
#[cfg(not(feature = "std"))]
pub const MAX_PENDING_DELIVERIES: usize = 65_536;

#[cfg(not(feature = "std"))]
const REGISTER_DOMAIN: &[u8] = b"edgerun:v0:relay:register";
#[cfg(not(feature = "std"))]
const SUBMIT_DOMAIN: &[u8] = b"edgerun:v0:relay:submit";
#[cfg(not(feature = "std"))]
const DELIVERY_RECEIPT_DOMAIN: &[u8] = b"edgerun:v0:relay:delivery-receipt";
#[cfg(not(feature = "std"))]
const REPORT_RECEIPT_DOMAIN: &[u8] = b"edgerun:v0:relay:delivery-report-receipt";

#[cfg(not(feature = "std"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayWireError {
    EmptyPacket,
    PacketTooLarge,
    InvalidPacket,
    #[cfg(feature = "wss")]
    TlsConfig,
}

#[cfg(not(feature = "std"))]
impl fmt::Display for RelayWireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPacket => write!(f, "empty relay packet"),
            Self::PacketTooLarge => write!(f, "relay packet too large"),
            Self::InvalidPacket => write!(f, "invalid relay packet"),
            #[cfg(feature = "wss")]
            Self::TlsConfig => write!(f, "invalid relay TLS configuration"),
        }
    }
}

#[cfg(all(not(feature = "std"), feature = "wss"))]
pub fn self_signed_wss_config(
    _hostnames: &[&str],
) -> Result<edgerun_rusttls::ServerConfig, RelayWireError> {
    edgerun_rusttls::ServerConfig::from_der_pair(
        BUILTIN_WSS_CERT_DER,
        BUILTIN_WSS_PRIVATE_KEY_DER,
    )
    .map_err(|_| RelayWireError::TlsConfig)
}

#[cfg(all(not(feature = "std"), feature = "wss"))]
const BUILTIN_WSS_CERT_DER: &[u8] = &[
    0x30, 0x82, 0x01, 0xd0, 0x30, 0x82, 0x01, 0x76, 0xa0, 0x03, 0x02, 0x01,
    0x02, 0x02, 0x14, 0x4d, 0xff, 0x15, 0x10, 0x14, 0xc4, 0x4b, 0xab, 0xb7,
    0x3a, 0xe2, 0xb7, 0x4d, 0x8a, 0xae, 0x1d, 0x94, 0x69, 0xe8, 0x75, 0x30,
    0x0a, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02, 0x30,
    0x22, 0x31, 0x20, 0x30, 0x1e, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x17,
    0x65, 0x64, 0x67, 0x65, 0x72, 0x75, 0x6e, 0x2d, 0x72, 0x65, 0x6c, 0x61,
    0x79, 0x2d, 0x77, 0x73, 0x73, 0x2e, 0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x30,
    0x1e, 0x17, 0x0d, 0x32, 0x36, 0x30, 0x35, 0x31, 0x30, 0x30, 0x31, 0x33,
    0x31, 0x33, 0x35, 0x5a, 0x17, 0x0d, 0x33, 0x36, 0x30, 0x35, 0x30, 0x37,
    0x30, 0x31, 0x33, 0x31, 0x33, 0x35, 0x5a, 0x30, 0x22, 0x31, 0x20, 0x30,
    0x1e, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x17, 0x65, 0x64, 0x67, 0x65,
    0x72, 0x75, 0x6e, 0x2d, 0x72, 0x65, 0x6c, 0x61, 0x79, 0x2d, 0x77, 0x73,
    0x73, 0x2e, 0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x30, 0x59, 0x30, 0x13, 0x06,
    0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a, 0x86,
    0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00, 0x04, 0x6e, 0xc1,
    0xd9, 0x29, 0x77, 0xbc, 0x9a, 0xc6, 0xf8, 0x19, 0x46, 0x1a, 0x34, 0x92,
    0xa2, 0x08, 0xca, 0x28, 0x58, 0xca, 0x93, 0x75, 0x9c, 0xc9, 0xf3, 0xed,
    0x50, 0x3b, 0x2c, 0x11, 0x52, 0xdd, 0x5c, 0x25, 0x70, 0xf6, 0xfb, 0x29,
    0xa5, 0x9a, 0x1f, 0xfd, 0x36, 0x9c, 0x8e, 0xfa, 0x3d, 0xd2, 0xff, 0x2e,
    0xcf, 0x78, 0x9b, 0xc0, 0x0d, 0x5b, 0xd3, 0x29, 0xb5, 0xe5, 0xca, 0x21,
    0x64, 0x3e, 0xa3, 0x81, 0x89, 0x30, 0x81, 0x86, 0x30, 0x1d, 0x06, 0x03,
    0x55, 0x1d, 0x0e, 0x04, 0x16, 0x04, 0x14, 0x95, 0x6c, 0x7c, 0x83, 0xfd,
    0x88, 0x5d, 0xd0, 0x95, 0x64, 0x8d, 0xdc, 0xe7, 0xd7, 0x25, 0x90, 0x6b,
    0xd0, 0x3a, 0x61, 0x30, 0x1f, 0x06, 0x03, 0x55, 0x1d, 0x23, 0x04, 0x18,
    0x30, 0x16, 0x80, 0x14, 0x95, 0x6c, 0x7c, 0x83, 0xfd, 0x88, 0x5d, 0xd0,
    0x95, 0x64, 0x8d, 0xdc, 0xe7, 0xd7, 0x25, 0x90, 0x6b, 0xd0, 0x3a, 0x61,
    0x30, 0x0f, 0x06, 0x03, 0x55, 0x1d, 0x13, 0x01, 0x01, 0xff, 0x04, 0x05,
    0x30, 0x03, 0x01, 0x01, 0xff, 0x30, 0x33, 0x06, 0x03, 0x55, 0x1d, 0x11,
    0x04, 0x2c, 0x30, 0x2a, 0x82, 0x09, 0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x68,
    0x6f, 0x73, 0x74, 0x82, 0x17, 0x65, 0x64, 0x67, 0x65, 0x72, 0x75, 0x6e,
    0x2d, 0x72, 0x65, 0x6c, 0x61, 0x79, 0x2d, 0x77, 0x73, 0x73, 0x2e, 0x6c,
    0x6f, 0x63, 0x61, 0x6c, 0x87, 0x04, 0x7f, 0x00, 0x00, 0x01, 0x30, 0x0a,
    0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02, 0x03, 0x48,
    0x00, 0x30, 0x45, 0x02, 0x21, 0x00, 0xe4, 0xf0, 0xdb, 0xc9, 0x81, 0x7c,
    0xbd, 0x62, 0x34, 0xae, 0xe0, 0x2e, 0x0e, 0x41, 0x48, 0xae, 0xed, 0xc9,
    0x92, 0x8e, 0xef, 0xc0, 0x5f, 0xa2, 0x6d, 0xfe, 0xa2, 0xb9, 0xbe, 0xd9,
    0xff, 0xd1, 0x02, 0x20, 0x69, 0x95, 0x30, 0x42, 0x6a, 0xc2, 0x06, 0x50,
    0x83, 0x1d, 0x57, 0xaf, 0x9f, 0x0f, 0x6a, 0x9b, 0x34, 0xd6, 0x85, 0x28,
    0x03, 0xfd, 0x5e, 0x2f, 0x85, 0x69, 0xf3, 0x6a, 0x39, 0xb1, 0x9b, 0xfe,
];

#[cfg(all(not(feature = "std"), feature = "wss"))]
const BUILTIN_WSS_PRIVATE_KEY_DER: &[u8] = &[
    0x30, 0x81, 0x87, 0x02, 0x01, 0x00, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86,
    0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d,
    0x03, 0x01, 0x07, 0x04, 0x6d, 0x30, 0x6b, 0x02, 0x01, 0x01, 0x04, 0x20,
    0xf0, 0x91, 0x71, 0xb8, 0x7e, 0x31, 0x0e, 0x84, 0xa4, 0x26, 0xdf, 0x0f,
    0xea, 0x47, 0xfd, 0xf8, 0x82, 0xfd, 0x0a, 0xba, 0xfb, 0xc8, 0x43, 0xa7,
    0xee, 0xc9, 0xa7, 0x80, 0x40, 0xf1, 0x69, 0x43, 0xa1, 0x44, 0x03, 0x42,
    0x00, 0x04, 0x6e, 0xc1, 0xd9, 0x29, 0x77, 0xbc, 0x9a, 0xc6, 0xf8, 0x19,
    0x46, 0x1a, 0x34, 0x92, 0xa2, 0x08, 0xca, 0x28, 0x58, 0xca, 0x93, 0x75,
    0x9c, 0xc9, 0xf3, 0xed, 0x50, 0x3b, 0x2c, 0x11, 0x52, 0xdd, 0x5c, 0x25,
    0x70, 0xf6, 0xfb, 0x29, 0xa5, 0x9a, 0x1f, 0xfd, 0x36, 0x9c, 0x8e, 0xfa,
    0x3d, 0xd2, 0xff, 0x2e, 0xcf, 0x78, 0x9b, 0xc0, 0x0d, 0x5b, 0xd3, 0x29,
    0xb5, 0xe5, 0xca, 0x21, 0x64, 0x3e,
];

#[cfg(not(feature = "std"))]
pub fn verify_register(register: &RelayRegister) -> bool {
    register.abi_version == RELAY_WIRE_ABI_VERSION
        && identity_shape_ok(&register.node)
        && signature_shape_ok(&register.signature)
        && signature_matches_identity(&register.node, &register.signature)
        && verify_signature(&register.signature, &register_preimage(register))
}

#[cfg(not(feature = "std"))]
pub fn verify_submit(submit: &RelaySubmit) -> bool {
    submit.abi_version == RELAY_WIRE_ABI_VERSION
        && submit.payload.len() <= MAX_PAYLOAD_LEN
        && sha256_array(&submit.payload) == submit.payload_sha256
        && identity_shape_ok(&submit.from)
        && identity_shape_ok(&submit.to)
        && signature_shape_ok(&submit.signature)
        && signature_matches_identity(&submit.from, &submit.signature)
        && verify_signature(&submit.signature, &submit_preimage(submit))
}

#[cfg(not(feature = "std"))]
pub fn verify_delivery_receipt(receipt: &RelayDeliveryReceipt) -> bool {
    receipt.abi_version == RELAY_WIRE_ABI_VERSION
        && matches!(
            receipt.status,
            RELAY_DELIVERY_STATUS_ACCEPTED | edgerun_wire::RELAY_DELIVERY_STATUS_REJECTED
        )
        && identity_shape_ok(&receipt.recipient)
        && signature_shape_ok(&receipt.signature)
        && signature_matches_identity(&receipt.recipient, &receipt.signature)
        && verify_signature(&receipt.signature, &delivery_receipt_preimage(receipt))
}

#[cfg(not(feature = "std"))]
pub fn verify_report_receipt(receipt: &RelayDeliveryReportReceipt) -> bool {
    receipt.abi_version == RELAY_WIRE_ABI_VERSION
        && matches!(
            receipt.status,
            RELAY_REPORT_STATUS_ACCEPTED | edgerun_wire::RELAY_REPORT_STATUS_REJECTED
        )
        && identity_shape_ok(&receipt.sender)
        && signature_shape_ok(&receipt.signature)
        && signature_matches_identity(&receipt.sender, &receipt.signature)
        && verify_signature(&receipt.signature, &report_receipt_preimage(receipt))
}

#[cfg(not(feature = "std"))]
fn identity_shape_ok(identity: &RelayIdentity) -> bool {
    match identity.algorithm {
        SIGNATURE_ALGORITHM_ED25519 => identity.public_key.len() == 32,
        SIGNATURE_ALGORITHM_ECDSA_P256_SHA256 => matches!(identity.public_key.len(), 33 | 64 | 65),
        _ => false,
    }
}

#[cfg(not(feature = "std"))]
fn signature_shape_ok(signature: &RelaySignature) -> bool {
    match signature.algorithm {
        SIGNATURE_ALGORITHM_ED25519 => {
            signature.public_key.len() == 32 && signature.signature.len() == 64
        }
        SIGNATURE_ALGORITHM_ECDSA_P256_SHA256 => {
            matches!(signature.public_key.len(), 33 | 64 | 65) && signature.signature.len() == 64
        }
        _ => false,
    }
}

#[cfg(not(feature = "std"))]
fn signature_matches_identity(identity: &RelayIdentity, signature: &RelaySignature) -> bool {
    identity.algorithm == signature.algorithm && identity.public_key == signature.public_key
}

#[cfg(not(feature = "std"))]
fn verify_signature(signature: &RelaySignature, preimage: &[u8]) -> bool {
    #[cfg(feature = "p256-message")]
    {
        return verify_message_signature(
            signature.algorithm,
            &signature.public_key,
            preimage,
            &signature.signature,
        )
        .is_ok();
    }

    #[cfg(not(feature = "p256-message"))]
    {
        let _ = (signature, preimage);
        false
    }
}

#[cfg(not(feature = "std"))]
pub fn register_preimage(register: &RelayRegister) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(REGISTER_DOMAIN);
    out.push(0);
    encode_identity(&mut out, &register.node);
    out.extend_from_slice(&register.sequence.to_be_bytes());
    out.extend_from_slice(&register.log_head);
    out
}

#[cfg(not(feature = "std"))]
pub fn submit_preimage(submit: &RelaySubmit) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(SUBMIT_DOMAIN);
    out.push(0);
    out.extend_from_slice(&submit.message_id);
    encode_identity(&mut out, &submit.from);
    encode_identity(&mut out, &submit.to);
    out.extend_from_slice(&submit.sequence.to_be_bytes());
    out.extend_from_slice(&submit.payload_sha256);
    out.extend_from_slice(&(submit.payload.len() as u64).to_be_bytes());
    out.extend_from_slice(&submit.payload);
    out
}

#[cfg(not(feature = "std"))]
pub fn delivery_receipt_preimage(receipt: &RelayDeliveryReceipt) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(DELIVERY_RECEIPT_DOMAIN);
    out.push(0);
    out.extend_from_slice(&receipt.message_id);
    encode_identity(&mut out, &receipt.recipient);
    out.extend_from_slice(&receipt.status.to_be_bytes());
    out.extend_from_slice(&receipt.recipient_sequence.to_be_bytes());
    out.extend_from_slice(&receipt.recipient_log_head);
    out.extend_from_slice(&receipt.request_sha256);
    out
}

#[cfg(not(feature = "std"))]
pub fn report_receipt_preimage(receipt: &RelayDeliveryReportReceipt) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(REPORT_RECEIPT_DOMAIN);
    out.push(0);
    out.extend_from_slice(&receipt.message_id);
    encode_identity(&mut out, &receipt.sender);
    out.extend_from_slice(&receipt.status.to_be_bytes());
    out.extend_from_slice(&receipt.sender_sequence.to_be_bytes());
    out.extend_from_slice(&receipt.sender_log_head);
    out.extend_from_slice(&receipt.report_sha256);
    out
}

#[cfg(not(feature = "std"))]
pub fn request_hash(request: &RelayDeliveryRequest) -> [u8; 32] {
    sha256_array(&relay_message_bytes(&RelayMessage::DeliveryRequest(request.clone())).unwrap())
}

#[cfg(not(feature = "std"))]
pub fn report_hash(report: &RelayDeliveryReport) -> [u8; 32] {
    sha256_array(&relay_message_bytes(&RelayMessage::DeliveryReport(report.clone())).unwrap())
}

#[cfg(not(feature = "std"))]
pub fn sha256_array(bytes: &[u8]) -> [u8; 32] {
    let digest = sha256(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

#[cfg(not(feature = "std"))]
fn encode_identity(out: &mut Vec<u8>, identity: &RelayIdentity) {
    out.extend_from_slice(&identity.algorithm.to_be_bytes());
    out.extend_from_slice(&(identity.public_key.len() as u64).to_be_bytes());
    out.extend_from_slice(&identity.public_key);
}

#[cfg(not(feature = "std"))]
pub fn decode_packet(bytes: &[u8]) -> Result<RelayMessage, RelayWireError> {
    if bytes.is_empty() {
        return Err(RelayWireError::EmptyPacket);
    }
    if bytes.len() > MAX_FRAME_LEN {
        return Err(RelayWireError::PacketTooLarge);
    }
    relay_message_from_bytes(bytes).map_err(|_| RelayWireError::InvalidPacket)
}

#[cfg(not(feature = "std"))]
pub fn encode_packet(message: &RelayMessage) -> Result<Vec<u8>, RelayWireError> {
    let bytes = relay_message_bytes(message).map_err(|_| RelayWireError::InvalidPacket)?;
    if bytes.len() > MAX_FRAME_LEN {
        return Err(RelayWireError::PacketTooLarge);
    }
    Ok(bytes)
}

#[cfg(all(test, not(feature = "std")))]
mod tests {
    extern crate std;

    use alloc::vec;
    use alloc::vec::Vec;
    use edgerun_crypto::{Ed25519SigningKey, Signer};
    use edgerun_wire::{
        RELAY_WIRE_ABI_VERSION, RelayIdentity, RelayMessage, RelayRegister, RelaySignature,
        RelaySubmit, SIGNATURE_ALGORITHM_ED25519,
    };

    #[cfg(feature = "virtio")]
    use edgerun_protocols::ethernet_ipv4::{
        ETH_TYPE_IPV4, IP_PROTO_TCP, IpAddr, IpStack, Network, ParsedPacket, TCP_FLAG_ACK,
        TCP_FLAG_PSH, TCP_FLAG_SYN, TcpHeader, checksum,
    };

    use crate::nostd_relay::{RelayEngine, RelayOutput};
    #[cfg(feature = "virtio")]
    use crate::nostd_virtio::{EthernetRelay, EthernetRelayEventKind};
    use crate::{register_preimage, sha256_array, submit_preimage};

    fn ed25519_identity(seed: u8) -> (Ed25519SigningKey, RelayIdentity) {
        let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
        let identity = RelayIdentity {
            algorithm: SIGNATURE_ALGORITHM_ED25519,
            public_key: key.verifying_key().as_bytes().to_vec(),
        };
        (key, identity)
    }

    fn sign_ed25519(
        key: &Ed25519SigningKey,
        identity: &RelayIdentity,
        preimage: &[u8],
    ) -> RelaySignature {
        RelaySignature {
            algorithm: identity.algorithm,
            public_key: identity.public_key.clone(),
            signature: key.sign(preimage).to_bytes().to_vec(),
        }
    }

    fn register_ed25519(seed: u8, sequence: u64) -> RelayRegister {
        let (key, node) = ed25519_identity(seed);
        let mut register = RelayRegister {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            node,
            sequence,
            log_head: [0xA5; 32],
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        register.signature = sign_ed25519(&key, &register.node, &register_preimage(&register));
        register
    }

    fn submit_ed25519(
        sender_seed: u8,
        to: RelayIdentity,
        message_id: [u8; 32],
        payload: &[u8],
    ) -> RelaySubmit {
        let (key, from) = ed25519_identity(sender_seed);
        let mut submit = RelaySubmit {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            message_id,
            from,
            to,
            sequence: 7,
            payload_sha256: sha256_array(payload),
            payload: payload.to_vec(),
            signature: RelaySignature {
                algorithm: 0,
                public_key: Vec::new(),
                signature: Vec::new(),
            },
        };
        submit.signature = sign_ed25519(&key, &submit.from, &submit_preimage(&submit));
        submit
    }

    #[test]
    fn no_std_engine_registers_and_forwards_submit() {
        let mut engine = RelayEngine::<u16>::new();
        let register = register_ed25519(2, 1);
        let recipient = register.node.clone();

        let outputs = engine.handle_message(10, RelayMessage::Register(register));
        assert!(matches!(
            &outputs[..],
            [RelayOutput::Ack {
                peer: 10,
                ack
            }] if ack.ok && ack.code == 200
        ));

        let submit = submit_ed25519(3, recipient, [0x11; 32], b"hello");
        let outputs = engine.handle_message(20, RelayMessage::Submit(submit));
        assert!(matches!(
            &outputs[..],
            [
                RelayOutput::Message {
                    peer: 10,
                    message: RelayMessage::DeliveryRequest(_)
                },
                RelayOutput::Ack {
                    peer: 20,
                    ack
                }
            ] if ack.ok && ack.code == 202
        ));
        assert_eq!(engine.pending_len(), 1);
    }

    #[cfg(feature = "wss")]
    #[test]
    fn no_std_wss_self_signed_config_builds() {
        let config = crate::self_signed_wss_config(&["localhost"]).expect("wss config");
        assert!(!config.certificate().cert_der.is_empty());
        assert_eq!(config.certificate().cert_chain_der.len(), 1);
    }

    #[cfg(feature = "virtio")]
    #[test]
    fn no_std_ethernet_relay_registers_and_forwards_udp() {
        let relay_mac = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
        let relay_ip = IpAddr::new(10, 0, 2, 15);
        let netmask = IpAddr::new(255, 255, 255, 0);
        let gateway = IpAddr::new(10, 0, 2, 2);
        let mut relay = EthernetRelay::new(relay_ip, netmask, gateway, relay_mac, 7373);

        let dest_mac = [0x02, 0, 0, 0, 0, 10];
        let dest_ip = IpAddr::new(10, 0, 2, 20);
        let dest_register = register_ed25519(2, 1);
        let dest_identity = dest_register.node.clone();
        let register_frame = udp_frame(
            dest_mac,
            dest_ip,
            relay_mac,
            relay_ip,
            40000,
            7373,
            &RelayMessage::Register(dest_register),
        );

        let outputs = relay.handle_frame(&register_frame);
        assert_eq!(relay.registered_len(), 1);
        assert_eq!(outputs.len(), 1);
        assert!(matches!(
            parse_udp_relay_message(&outputs[0]),
            RelayMessage::Ack(ack) if ack.ok && ack.code == 200
        ));

        let src_mac = [0x02, 0, 0, 0, 0, 20];
        let src_ip = IpAddr::new(10, 0, 2, 21);
        let submit = submit_ed25519(3, dest_identity, [0x44; 32], b"hello virtio udp");
        let submit_frame = udp_frame(
            src_mac,
            src_ip,
            relay_mac,
            relay_ip,
            40001,
            7373,
            &RelayMessage::Submit(submit),
        );

        let outputs = relay.handle_frame(&submit_frame);
        assert_eq!(relay.pending_len(), 1);
        assert_eq!(outputs.len(), 2);
        assert!(matches!(
            parse_udp_relay_message(&outputs[0]),
            RelayMessage::DeliveryRequest(_)
        ));
        assert!(matches!(
            parse_udp_relay_message(&outputs[1]),
            RelayMessage::Ack(ack) if ack.ok && ack.code == 202
        ));
    }

    #[cfg(feature = "virtio")]
    #[test]
    fn no_std_ethernet_relay_registers_and_forwards_tcp() {
        let relay_mac = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
        let relay_ip = IpAddr::new(10, 0, 2, 15);
        let netmask = IpAddr::new(255, 255, 255, 0);
        let gateway = IpAddr::new(10, 0, 2, 2);
        let mut relay = EthernetRelay::new(relay_ip, netmask, gateway, relay_mac, 7373);

        let dest_mac = [0x02, 0, 0, 0, 0, 10];
        let dest_ip = IpAddr::new(10, 0, 2, 20);
        let dest_port = 40000;
        let syn = tcp_frame(
            dest_mac,
            *dest_ip.as_bytes(),
            relay_mac,
            *relay_ip.as_bytes(),
            dest_port,
            7373,
            1000,
            0,
            TCP_FLAG_SYN,
            &[],
        );
        let outputs = relay.handle_frame_at(&syn, 1);
        assert_eq!(outputs.len(), 1);
        let (syn_ack, payload) = parse_tcp_frame(&outputs[0]);
        assert_eq!(payload.len(), 0);
        assert_eq!(
            syn_ack.flags & (TCP_FLAG_SYN | TCP_FLAG_ACK),
            TCP_FLAG_SYN | TCP_FLAG_ACK
        );
        assert_eq!(syn_ack.ack, 1001);

        let register = register_ed25519(2, 1);
        let dest_identity = register.node.clone();
        let register_payload = framed_relay_payload(&RelayMessage::Register(register));
        let register_frame = tcp_frame(
            dest_mac,
            *dest_ip.as_bytes(),
            relay_mac,
            *relay_ip.as_bytes(),
            dest_port,
            7373,
            1001,
            syn_ack.seq.wrapping_add(1),
            TCP_FLAG_ACK | TCP_FLAG_PSH,
            &register_payload,
        );
        let outputs = relay.handle_frame_at(&register_frame, 2);
        assert_eq!(relay.registered_len(), 1);
        assert!(outputs.iter().any(|frame| matches!(
            parse_tcp_relay_message(frame),
            Some(RelayMessage::Ack(ack)) if ack.ok && ack.code == 200
        )));

        let src_mac = [0x02, 0, 0, 0, 0, 20];
        let src_ip = IpAddr::new(10, 0, 2, 21);
        let submit = submit_ed25519(3, dest_identity, [0x55; 32], b"hello virtio tcp");
        let submit_frame = udp_frame(
            src_mac,
            src_ip,
            relay_mac,
            relay_ip,
            40001,
            7373,
            &RelayMessage::Submit(submit),
        );

        let outputs = relay.handle_frame_at(&submit_frame, 3);
        assert_eq!(relay.pending_len(), 1);
        assert!(outputs.iter().any(|frame| matches!(
            parse_tcp_relay_message(frame),
            Some(RelayMessage::DeliveryRequest(_))
        )));
        assert!(outputs.iter().any(|frame| matches!(
            parse_udp_relay_message_opt(frame),
            Some(RelayMessage::Ack(ack)) if ack.ok && ack.code == 202
        )));
    }

    #[cfg(feature = "virtio")]
    #[test]
    fn no_std_ethernet_relay_expires_tcp_after_one_minute() {
        let relay_mac = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
        let relay_ip = IpAddr::new(10, 0, 2, 15);
        let mut relay = EthernetRelay::new(
            relay_ip,
            IpAddr::new(255, 255, 255, 0),
            IpAddr::new(10, 0, 2, 2),
            relay_mac,
            7373,
        );
        let peer_mac = [0x02, 0, 0, 0, 0, 30];
        let peer_ip = IpAddr::new(10, 0, 2, 30);
        let syn = tcp_frame(
            peer_mac,
            *peer_ip.as_bytes(),
            relay_mac,
            *relay_ip.as_bytes(),
            40002,
            7373,
            2000,
            0,
            TCP_FLAG_SYN,
            &[],
        );
        let mut events = Vec::new();
        let outputs = relay.handle_frame_at_with_events(&syn, 10, &mut events);
        assert_eq!(outputs.len(), 1);
        assert!(matches!(
            events.as_slice(),
            [event] if event.kind == EthernetRelayEventKind::TcpOpened
        ));

        events.clear();
        let outputs = relay.poll_tcp_with_events(60_010, &mut events);
        assert_eq!(outputs.len(), 1);
        assert!(matches!(
            events.as_slice(),
            [event] if event.kind == EthernetRelayEventKind::TcpClosed {
                reason: "lifetime_exceeded"
            }
        ));
    }

    #[cfg(all(feature = "virtio", feature = "wss"))]
    #[test]
    fn no_std_virtio_relay_keeps_wss_cert_in_memory() {
        let relay_mac = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
        let relay_ip = IpAddr::new(10, 0, 2, 15);
        let relay = EthernetRelay::new(
            relay_ip,
            IpAddr::new(255, 255, 255, 0),
            IpAddr::new(10, 0, 2, 2),
            relay_mac,
            7373,
        );
        let config = crate::self_signed_wss_config(&["localhost"]).expect("wss config");
        let virtio = crate::nostd_virtio::VirtioRelay::with_wss_config(relay, config);
        assert!(virtio.wss_config().is_some());
        assert!(
            !virtio
                .wss_config()
                .expect("wss config")
                .certificate()
                .cert_der
                .is_empty()
        );
    }

    #[cfg(feature = "virtio")]
    fn udp_frame(
        src_mac: [u8; 6],
        src_ip: IpAddr,
        dst_mac: [u8; 6],
        dst_ip: IpAddr,
        src_port: u16,
        dst_port: u16,
        message: &RelayMessage,
    ) -> Vec<u8> {
        let bytes = crate::encode_packet(message).expect("encode relay message");
        let mut stack = IpStack::new();
        stack.configure(
            src_ip,
            IpAddr::new(255, 255, 255, 0),
            IpAddr::zero(),
            src_mac,
        );
        let mut network = Network::new(&mut stack);
        network
            .send_udp_eth(dst_mac, dst_ip, src_port, dst_port, &bytes)
            .expect("udp frame")
            .to_vec()
    }

    #[cfg(feature = "virtio")]
    fn parse_udp_relay_message(frame: &[u8]) -> RelayMessage {
        parse_udp_relay_message_opt(frame).expect("expected udp")
    }

    #[cfg(feature = "virtio")]
    fn parse_udp_relay_message_opt(frame: &[u8]) -> Option<RelayMessage> {
        let mut stack = IpStack::new();
        let mut network = Network::new(&mut stack);
        match network.recv(frame)? {
            ParsedPacket::Udp { payload, .. } => crate::decode_packet(payload).ok(),
            _ => None,
        }
    }

    #[cfg(feature = "virtio")]
    #[allow(clippy::too_many_arguments)]
    fn tcp_frame(
        src_mac: [u8; 6],
        src_ip: [u8; 4],
        dst_mac: [u8; 6],
        dst_ip: [u8; 4],
        src_port: u16,
        dst_port: u16,
        seq: u32,
        ack: u32,
        flags: u8,
        payload: &[u8],
    ) -> Vec<u8> {
        let tcp_len = 20 + payload.len();
        let ip_len = 20 + tcp_len;
        let mut frame = vec![0u8; 14 + ip_len];
        frame[0..6].copy_from_slice(&dst_mac);
        frame[6..12].copy_from_slice(&src_mac);
        write_u16(&mut frame, 12, ETH_TYPE_IPV4);

        let ip_start = 14;
        frame[ip_start] = 0x45;
        write_u16(&mut frame, ip_start + 2, ip_len as u16);
        frame[ip_start + 8] = 64;
        frame[ip_start + 9] = IP_PROTO_TCP;
        frame[ip_start + 12..ip_start + 16].copy_from_slice(&src_ip);
        frame[ip_start + 16..ip_start + 20].copy_from_slice(&dst_ip);
        let ip_sum = checksum(&frame[ip_start..ip_start + 20]);
        write_u16(&mut frame, ip_start + 10, ip_sum);

        let tcp_start = ip_start + 20;
        write_u16(&mut frame, tcp_start, src_port);
        write_u16(&mut frame, tcp_start + 2, dst_port);
        write_u32(&mut frame, tcp_start + 4, seq);
        write_u32(&mut frame, tcp_start + 8, ack);
        frame[tcp_start + 12] = 5 << 4;
        frame[tcp_start + 13] = flags;
        write_u16(&mut frame, tcp_start + 14, 64240);
        frame[tcp_start + 20..tcp_start + 20 + payload.len()].copy_from_slice(payload);
        let tcp_sum = tcp_checksum(&src_ip, &dst_ip, &frame[tcp_start..tcp_start + tcp_len]);
        write_u16(&mut frame, tcp_start + 16, tcp_sum);
        frame
    }

    #[cfg(feature = "virtio")]
    fn framed_relay_payload(message: &RelayMessage) -> Vec<u8> {
        let bytes = crate::encode_packet(message).expect("encode relay message");
        let mut out = Vec::with_capacity(4 + bytes.len());
        out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        out.extend_from_slice(&bytes);
        out
    }

    #[cfg(feature = "virtio")]
    fn parse_tcp_relay_message(frame: &[u8]) -> Option<RelayMessage> {
        let (_, payload) = parse_tcp_frame(frame);
        if payload.len() < 4 {
            return None;
        }
        let len = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
        if payload.len() != 4 + len {
            return None;
        }
        crate::decode_packet(&payload[4..]).ok()
    }

    #[cfg(feature = "virtio")]
    fn parse_tcp_frame(frame: &[u8]) -> (TcpHeader, Vec<u8>) {
        let mut stack = IpStack::new();
        let mut network = Network::new(&mut stack);
        match network.recv(frame).expect("parse frame") {
            ParsedPacket::Tcp {
                header, payload, ..
            } => (header, payload.to_vec()),
            _ => panic!("expected tcp"),
        }
    }

    #[cfg(feature = "virtio")]
    fn tcp_checksum(src_ip: &[u8; 4], dst_ip: &[u8; 4], tcp_segment: &[u8]) -> u16 {
        let mut pseudo = Vec::with_capacity(12 + tcp_segment.len());
        pseudo.extend_from_slice(src_ip);
        pseudo.extend_from_slice(dst_ip);
        pseudo.push(0);
        pseudo.push(IP_PROTO_TCP);
        pseudo.extend_from_slice(&(tcp_segment.len() as u16).to_be_bytes());
        pseudo.extend_from_slice(tcp_segment);
        checksum(&pseudo)
    }

    #[cfg(feature = "virtio")]
    fn write_u16(out: &mut [u8], offset: usize, value: u16) {
        out[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    #[cfg(feature = "virtio")]
    fn write_u32(out: &mut [u8], offset: usize, value: u32) {
        out[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }
}
