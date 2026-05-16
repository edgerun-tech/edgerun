use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::codec::{EdgeWire, WireCursor, WireWriter};
use crate::preimage::{HashBuilder, PreimageBuilder};
use crate::protocol::{
    Hash, NODE_ROLE_NOTARY, NODE_ROLE_STORAGE, NODE_ROLE_VERIFIER, NodeId, NodeIdentity,
    WORK_WIRE_ABI_VERSION, WorkProtocolError, WorkSignature,
};
use crate::signing::{sign_ed25519, verify_signature};

const PACKET_TRANSIT_DOMAIN: &[u8] = b"edgerun:v1:work:packet-transit";
const PACKET_TRANSIT_CHAIN_DOMAIN: &[u8] = b"edgerun:v1:work:packet-transit-chain";
const RELAY_DELIVERY_OUTPUT_DOMAIN: &[u8] = b"edgerun:v1:work:relay-delivery-output";
const RELAY_TRANSIT_BUNDLE_DOMAIN: &[u8] = b"edgerun:v1:work:relay-transit-bundle";
const RELAY_TRANSIT_ROUTE_COMMITMENT_DOMAIN: &[u8] =
    b"edgerun:v1:work:relay-transit-route-commitment";
const RELAY_PROOF_CUSTODY_ACK_DOMAIN: &[u8] = b"edgerun:v1:work:relay-proof-custody-ack";

pub const MAX_RELAY_TRANSIT_BUNDLE_HOPS: usize = 64;
pub const RELAY_PROOF_CUSTODY_KIND_PACKED: u16 = 1;
pub const RELAY_PROOF_CUSTODY_KIND_NOTARIZED: u16 = 2;
pub const RELAY_PROOF_CUSTODY_KIND_STORED: u16 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketTransitHashInput {
    pub node_id: NodeId,
    pub from: NodeId,
    pub to: NodeId,
    pub channel_id: Hash,
    pub route_hash: Hash,
    pub packet_hash: Hash,
    pub sequence: u64,
    pub previous_transit_hash: Hash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelayTransitHopEvidence {
    pub hop_index: u64,
    pub relay_node_id: NodeId,
    pub from: NodeId,
    pub to: NodeId,
    pub channel_id: Hash,
    pub route_hash: Hash,
    pub input_hash: Hash,
    pub packet_hash: Hash,
    pub sequence: u64,
    pub previous_transit_hash: Hash,
    pub transit_hash: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayTransitBundle {
    pub abi_version: u16,
    pub request_hash: Hash,
    pub admission_hash: Hash,
    pub controlling_admission_node_id: NodeId,
    pub source_node_id: NodeId,
    pub destination_node_id: NodeId,
    pub relay_path: Vec<NodeId>,
    pub packet_hash: Hash,
    pub final_delivery_proof_hash: Hash,
    pub hops: Vec<RelayTransitHopEvidence>,
    pub bundle_root: Hash,
}

pub struct RelayTransitBundleContext<'a> {
    pub request_hash: Hash,
    pub admission_hash: Hash,
    pub controlling_admission_node_id: NodeId,
    pub source_node_id: NodeId,
    pub destination_node_id: NodeId,
    pub relay_path: &'a [NodeId],
    pub packet_hash: Hash,
    pub final_delivery_proof_hash: Hash,
    pub max_hops: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayProofCustodyAck {
    pub abi_version: u16,
    pub custodian: NodeIdentity,
    pub request_hash: Hash,
    pub admission_hash: Hash,
    pub bundle_root: Hash,
    pub packed_proof_root: Hash,
    pub custody_kind: u16,
    pub sequence_start: u64,
    pub sequence_end: u64,
    pub valid_until_unix_ms: u64,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayTransitBundleBuilder {
    request_hash: Hash,
    admission_hash: Hash,
    controlling_admission_node_id: NodeId,
    source_node_id: NodeId,
    destination_node_id: NodeId,
    relay_path: Vec<NodeId>,
    packet_hash: Hash,
    hops: Vec<RelayTransitHopEvidence>,
    max_hops: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelayTransitBundleError {
    BadAbi,
    EmptyPath,
    TooManyHops,
    ContextMismatch,
    PathMismatch,
    HopIndexMismatch,
    HopRelayMismatch,
    HopEndpointMismatch,
    PacketMismatch,
    TransitHashMismatch,
    BundleRootMismatch,
    MissingFinalDeliveryProof,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelayProofCustodyAckError {
    BadAbi,
    WrongCustodianRole,
    InvalidCustodyKind,
    CustodyKindRoleMismatch,
    EmptyPackedProofRoot,
    InvalidRange,
    EmptyChain,
    CustodyChainMismatch,
    CustodyChainDowngrade,
    InvalidSignature,
    BundleMismatch,
    SequenceWindowMismatch,
    Expired,
}

pub fn packet_transit_hash(input: &PacketTransitHashInput) -> Hash {
    HashBuilder::domain(PACKET_TRANSIT_DOMAIN)
        .node_id(&input.node_id)
        .node_id(&input.from)
        .node_id(&input.to)
        .hash(&input.channel_id)
        .hash(&input.route_hash)
        .hash(&input.packet_hash)
        .u64(input.sequence)
        .hash(&input.previous_transit_hash)
        .finish()
}

pub fn relay_transit_hop_evidence(
    input: PacketTransitHashInput,
    input_hash: Hash,
    hop_index: u64,
) -> RelayTransitHopEvidence {
    RelayTransitHopEvidence {
        hop_index,
        relay_node_id: input.node_id,
        from: input.from,
        to: input.to,
        channel_id: input.channel_id,
        route_hash: input.route_hash,
        input_hash,
        packet_hash: input.packet_hash,
        sequence: input.sequence,
        previous_transit_hash: input.previous_transit_hash,
        transit_hash: packet_transit_hash(&input),
    }
}

impl RelayTransitBundleBuilder {
    pub fn new(
        request_hash: Hash,
        admission_hash: Hash,
        controlling_admission_node_id: NodeId,
        source_node_id: NodeId,
        destination_node_id: NodeId,
        relay_path: Vec<NodeId>,
        packet_hash: Hash,
        max_hops: usize,
    ) -> Result<Self, RelayTransitBundleError> {
        let max_hops = if max_hops == 0 {
            MAX_RELAY_TRANSIT_BUNDLE_HOPS
        } else {
            max_hops
        };
        if relay_path.is_empty() {
            return Err(RelayTransitBundleError::EmptyPath);
        }
        if relay_path.len() > max_hops {
            return Err(RelayTransitBundleError::TooManyHops);
        }
        Ok(Self {
            request_hash,
            admission_hash,
            controlling_admission_node_id,
            source_node_id,
            destination_node_id,
            relay_path,
            packet_hash,
            hops: Vec::new(),
            max_hops,
        })
    }

    pub fn push_hop(
        &mut self,
        input: PacketTransitHashInput,
        input_hash: Hash,
    ) -> Result<Hash, RelayTransitBundleError> {
        let index = self.hops.len();
        if index >= self.max_hops || index >= self.relay_path.len() {
            return Err(RelayTransitBundleError::TooManyHops);
        }
        if input.node_id != self.relay_path[index] {
            return Err(RelayTransitBundleError::HopRelayMismatch);
        }
        let expected_from = if index == 0 {
            self.source_node_id
        } else {
            self.relay_path[index - 1]
        };
        let expected_to = if index + 1 == self.relay_path.len() {
            self.destination_node_id
        } else {
            self.relay_path[index + 1]
        };
        if input.from != expected_from || input.to != expected_to {
            return Err(RelayTransitBundleError::HopEndpointMismatch);
        }
        if input.packet_hash != self.packet_hash {
            return Err(RelayTransitBundleError::PacketMismatch);
        }
        let hop = relay_transit_hop_evidence(input, input_hash, index as u64);
        let transit_hash = hop.transit_hash;
        self.hops.push(hop);
        Ok(transit_hash)
    }

    pub fn route_commitment(&self) -> Hash {
        relay_transit_route_commitment(
            self.source_node_id,
            self.destination_node_id,
            &self.relay_path,
            &self.hops,
        )
    }

    pub fn finish(
        self,
        final_delivery_proof_hash: Hash,
    ) -> Result<RelayTransitBundle, RelayTransitBundleError> {
        if final_delivery_proof_hash == [0u8; 32] {
            return Err(RelayTransitBundleError::MissingFinalDeliveryProof);
        }
        if self.hops.is_empty() {
            return Err(RelayTransitBundleError::EmptyPath);
        }
        if self.hops.len() != self.relay_path.len() {
            return Err(RelayTransitBundleError::PathMismatch);
        }
        Ok(finalize_relay_transit_bundle(RelayTransitBundle {
            abi_version: WORK_WIRE_ABI_VERSION,
            request_hash: self.request_hash,
            admission_hash: self.admission_hash,
            controlling_admission_node_id: self.controlling_admission_node_id,
            source_node_id: self.source_node_id,
            destination_node_id: self.destination_node_id,
            relay_path: self.relay_path,
            packet_hash: self.packet_hash,
            final_delivery_proof_hash,
            hops: self.hops,
            bundle_root: [0u8; 32],
        }))
    }
}

pub fn relay_transit_bundle_root(bundle: &RelayTransitBundle) -> Hash {
    let mut builder = HashBuilder::domain(RELAY_TRANSIT_BUNDLE_DOMAIN)
        .u16(bundle.abi_version)
        .hash(&bundle.request_hash)
        .hash(&bundle.admission_hash)
        .node_id(&bundle.controlling_admission_node_id)
        .node_id(&bundle.source_node_id)
        .node_id(&bundle.destination_node_id)
        .u64(bundle.relay_path.len() as u64);
    for relay in &bundle.relay_path {
        builder = builder.node_id(relay);
    }
    builder = builder
        .hash(&bundle.packet_hash)
        .hash(&bundle.final_delivery_proof_hash)
        .u64(bundle.hops.len() as u64);
    for hop in &bundle.hops {
        builder = builder
            .u64(hop.hop_index)
            .node_id(&hop.relay_node_id)
            .node_id(&hop.from)
            .node_id(&hop.to)
            .hash(&hop.channel_id)
            .hash(&hop.route_hash)
            .hash(&hop.input_hash)
            .hash(&hop.packet_hash)
            .u64(hop.sequence)
            .hash(&hop.previous_transit_hash)
            .hash(&hop.transit_hash);
    }
    builder.finish()
}

pub fn relay_transit_route_commitment(
    source_node_id: NodeId,
    destination_node_id: NodeId,
    relay_path: &[NodeId],
    hops: &[RelayTransitHopEvidence],
) -> Hash {
    let mut builder = HashBuilder::domain(RELAY_TRANSIT_ROUTE_COMMITMENT_DOMAIN)
        .node_id(&source_node_id)
        .node_id(&destination_node_id)
        .u64(relay_path.len() as u64);
    for relay in relay_path {
        builder = builder.node_id(relay);
    }
    builder = builder.u64(hops.len() as u64);
    for hop in hops {
        builder = builder
            .u64(hop.hop_index)
            .node_id(&hop.relay_node_id)
            .node_id(&hop.from)
            .node_id(&hop.to)
            .hash(&hop.channel_id)
            .hash(&hop.route_hash);
    }
    builder.finish()
}

pub fn relay_transit_bundle_route_commitment(bundle: &RelayTransitBundle) -> Hash {
    relay_transit_route_commitment(
        bundle.source_node_id,
        bundle.destination_node_id,
        &bundle.relay_path,
        &bundle.hops,
    )
}

pub fn relay_proof_custody_ack_preimage(value: &RelayProofCustodyAck) -> Vec<u8> {
    PreimageBuilder::domain(RELAY_PROOF_CUSTODY_ACK_DOMAIN)
        .node(&value.custodian)
        .hash(&value.request_hash)
        .hash(&value.admission_hash)
        .hash(&value.bundle_root)
        .hash(&value.packed_proof_root)
        .u16(value.custody_kind)
        .u64(value.sequence_start)
        .u64(value.sequence_end)
        .u64(value.valid_until_unix_ms)
        .finish()
}

pub fn relay_proof_custody_ack_hash(value: &RelayProofCustodyAck) -> Hash {
    crate::codec::blake3_hash(&relay_proof_custody_ack_preimage(value))
}

pub fn sign_relay_proof_custody_ack(
    key: &Ed25519SigningKey,
    mut value: RelayProofCustodyAck,
) -> RelayProofCustodyAck {
    value.signature = sign_ed25519(key, &relay_proof_custody_ack_preimage(&value));
    value
}

pub fn sign_relay_proof_custody_ack_for_bundle(
    key: &Ed25519SigningKey,
    custodian: NodeIdentity,
    bundle: &RelayTransitBundle,
    packed_proof_root: Hash,
    custody_kind: u16,
    valid_until_unix_ms: u64,
) -> Result<RelayProofCustodyAck, RelayProofCustodyAckError> {
    if bundle.hops.is_empty() || bundle.bundle_root != relay_transit_bundle_root(bundle) {
        return Err(RelayProofCustodyAckError::BundleMismatch);
    }
    let ack = sign_relay_proof_custody_ack(
        key,
        RelayProofCustodyAck {
            abi_version: WORK_WIRE_ABI_VERSION,
            custodian,
            request_hash: bundle.request_hash,
            admission_hash: bundle.admission_hash,
            bundle_root: bundle.bundle_root,
            packed_proof_root,
            custody_kind,
            sequence_start: 1,
            sequence_end: bundle.hops.len() as u64,
            valid_until_unix_ms,
            signature: crate::signing::empty_signature(),
        },
    );
    verify_relay_proof_custody_ack_for_bundle(&ack, bundle, 0)?;
    Ok(ack)
}

pub fn verify_relay_proof_custody_ack(
    value: &RelayProofCustodyAck,
) -> Result<Hash, RelayProofCustodyAckError> {
    if value.abi_version != WORK_WIRE_ABI_VERSION {
        return Err(RelayProofCustodyAckError::BadAbi);
    }
    if !matches!(
        value.custodian.role,
        NODE_ROLE_STORAGE | NODE_ROLE_NOTARY | NODE_ROLE_VERIFIER
    ) {
        return Err(RelayProofCustodyAckError::WrongCustodianRole);
    }
    if !matches!(
        value.custody_kind,
        RELAY_PROOF_CUSTODY_KIND_PACKED
            | RELAY_PROOF_CUSTODY_KIND_NOTARIZED
            | RELAY_PROOF_CUSTODY_KIND_STORED
    ) {
        return Err(RelayProofCustodyAckError::InvalidCustodyKind);
    }
    let role_matches_kind = match value.custody_kind {
        RELAY_PROOF_CUSTODY_KIND_PACKED => {
            matches!(value.custodian.role, NODE_ROLE_STORAGE | NODE_ROLE_VERIFIER)
        }
        RELAY_PROOF_CUSTODY_KIND_NOTARIZED => value.custodian.role == NODE_ROLE_NOTARY,
        RELAY_PROOF_CUSTODY_KIND_STORED => value.custodian.role == NODE_ROLE_STORAGE,
        _ => false,
    };
    if !role_matches_kind {
        return Err(RelayProofCustodyAckError::CustodyKindRoleMismatch);
    }
    if value.packed_proof_root == [0u8; 32] {
        return Err(RelayProofCustodyAckError::EmptyPackedProofRoot);
    }
    if value.sequence_start == 0 || value.sequence_end < value.sequence_start {
        return Err(RelayProofCustodyAckError::InvalidRange);
    }
    if !verify_signature(
        &value.custodian,
        &value.signature,
        &relay_proof_custody_ack_preimage(value),
    ) {
        return Err(RelayProofCustodyAckError::InvalidSignature);
    }
    Ok(relay_proof_custody_ack_hash(value))
}

pub fn verify_relay_proof_custody_ack_for_bundle(
    value: &RelayProofCustodyAck,
    bundle: &RelayTransitBundle,
    now_unix_ms: u64,
) -> Result<Hash, RelayProofCustodyAckError> {
    let ack_hash = verify_relay_proof_custody_ack(value)?;
    if value.valid_until_unix_ms < now_unix_ms {
        return Err(RelayProofCustodyAckError::Expired);
    }
    if value.request_hash != bundle.request_hash
        || value.admission_hash != bundle.admission_hash
        || value.bundle_root != relay_transit_bundle_root(bundle)
        || value.bundle_root != bundle.bundle_root
    {
        return Err(RelayProofCustodyAckError::BundleMismatch);
    }
    if bundle.hops.is_empty() {
        return Err(RelayProofCustodyAckError::BundleMismatch);
    }
    let expected_start = 1;
    let expected_end = bundle.hops.len() as u64;
    if value.sequence_start != expected_start || value.sequence_end != expected_end {
        return Err(RelayProofCustodyAckError::SequenceWindowMismatch);
    }
    Ok(ack_hash)
}

pub fn verify_relay_proof_custody_chain(
    bundle: &RelayTransitBundle,
    acknowledgements: &[RelayProofCustodyAck],
    expected_packed_proof_root: Hash,
    required_final_custody_kind: u16,
    now_unix_ms: u64,
) -> Result<Hash, RelayProofCustodyAckError> {
    if acknowledgements.is_empty() {
        return Err(RelayProofCustodyAckError::EmptyChain);
    }
    if !matches!(
        required_final_custody_kind,
        RELAY_PROOF_CUSTODY_KIND_PACKED
            | RELAY_PROOF_CUSTODY_KIND_NOTARIZED
            | RELAY_PROOF_CUSTODY_KIND_STORED
    ) {
        return Err(RelayProofCustodyAckError::InvalidCustodyKind);
    }

    let mut previous_rank = 0;
    let mut final_hash = [0u8; 32];
    for ack in acknowledgements {
        if ack.packed_proof_root != expected_packed_proof_root {
            return Err(RelayProofCustodyAckError::CustodyChainMismatch);
        }
        final_hash = verify_relay_proof_custody_ack_for_bundle(ack, bundle, now_unix_ms)?;
        let rank = relay_proof_custody_kind_rank(ack.custody_kind)?;
        if rank != previous_rank + 1 {
            return Err(RelayProofCustodyAckError::CustodyChainDowngrade);
        }
        previous_rank = rank;
    }

    let final_ack = acknowledgements
        .last()
        .ok_or(RelayProofCustodyAckError::EmptyChain)?;
    if final_ack.custody_kind != required_final_custody_kind {
        return Err(RelayProofCustodyAckError::CustodyChainMismatch);
    }
    Ok(final_hash)
}

fn relay_proof_custody_kind_rank(custody_kind: u16) -> Result<u8, RelayProofCustodyAckError> {
    match custody_kind {
        RELAY_PROOF_CUSTODY_KIND_PACKED => Ok(1),
        RELAY_PROOF_CUSTODY_KIND_NOTARIZED => Ok(2),
        RELAY_PROOF_CUSTODY_KIND_STORED => Ok(3),
        _ => Err(RelayProofCustodyAckError::InvalidCustodyKind),
    }
}

pub fn finalize_relay_transit_bundle(mut bundle: RelayTransitBundle) -> RelayTransitBundle {
    bundle.bundle_root = relay_transit_bundle_root(&bundle);
    bundle
}

pub fn verify_relay_transit_bundle(
    bundle: &RelayTransitBundle,
    context: &RelayTransitBundleContext<'_>,
) -> Result<Hash, RelayTransitBundleError> {
    if bundle.abi_version != WORK_WIRE_ABI_VERSION {
        return Err(RelayTransitBundleError::BadAbi);
    }
    let max_hops = if context.max_hops == 0 {
        MAX_RELAY_TRANSIT_BUNDLE_HOPS
    } else {
        context.max_hops
    };
    if bundle.hops.is_empty() || bundle.relay_path.is_empty() {
        return Err(RelayTransitBundleError::EmptyPath);
    }
    if bundle.hops.len() > max_hops || bundle.relay_path.len() > max_hops {
        return Err(RelayTransitBundleError::TooManyHops);
    }
    if bundle.request_hash != context.request_hash
        || bundle.admission_hash != context.admission_hash
        || bundle.controlling_admission_node_id != context.controlling_admission_node_id
        || bundle.source_node_id != context.source_node_id
        || bundle.destination_node_id != context.destination_node_id
        || bundle.packet_hash != context.packet_hash
        || bundle.final_delivery_proof_hash != context.final_delivery_proof_hash
    {
        return Err(RelayTransitBundleError::ContextMismatch);
    }
    if bundle.relay_path.as_slice() != context.relay_path
        || bundle.hops.len() != bundle.relay_path.len()
    {
        return Err(RelayTransitBundleError::PathMismatch);
    }
    for (index, hop) in bundle.hops.iter().enumerate() {
        if hop.hop_index != index as u64 {
            return Err(RelayTransitBundleError::HopIndexMismatch);
        }
        if hop.relay_node_id != bundle.relay_path[index] {
            return Err(RelayTransitBundleError::HopRelayMismatch);
        }
        let expected_from = if index == 0 {
            bundle.source_node_id
        } else {
            bundle.relay_path[index - 1]
        };
        let expected_to = if index + 1 == bundle.hops.len() {
            bundle.destination_node_id
        } else {
            bundle.relay_path[index + 1]
        };
        if hop.from != expected_from || hop.to != expected_to {
            return Err(RelayTransitBundleError::HopEndpointMismatch);
        }
        if hop.packet_hash != bundle.packet_hash {
            return Err(RelayTransitBundleError::PacketMismatch);
        }
        let expected_transit_hash = packet_transit_hash(&PacketTransitHashInput {
            node_id: hop.relay_node_id,
            from: hop.from,
            to: hop.to,
            channel_id: hop.channel_id,
            route_hash: hop.route_hash,
            packet_hash: hop.packet_hash,
            sequence: hop.sequence,
            previous_transit_hash: hop.previous_transit_hash,
        });
        if hop.transit_hash != expected_transit_hash {
            return Err(RelayTransitBundleError::TransitHashMismatch);
        }
    }
    let expected_root = relay_transit_bundle_root(bundle);
    if bundle.bundle_root != expected_root {
        return Err(RelayTransitBundleError::BundleRootMismatch);
    }
    Ok(expected_root)
}

pub fn relay_delivery_output_hash(
    transit_hash: Hash,
    forwarded_packet_hash: Hash,
    receiver_channel_proof_hash: Hash,
) -> Hash {
    HashBuilder::domain(RELAY_DELIVERY_OUTPUT_DOMAIN)
        .hash(&transit_hash)
        .hash(&forwarded_packet_hash)
        .hash(&receiver_channel_proof_hash)
        .finish()
}

pub fn packet_transit_chain_hash(transit_hashes: &[Hash]) -> Hash {
    let mut builder =
        HashBuilder::domain(PACKET_TRANSIT_CHAIN_DOMAIN).u64(transit_hashes.len() as u64);
    for hash in transit_hashes {
        builder = builder.hash(hash);
    }
    builder.finish()
}

impl EdgeWire for RelayTransitHopEvidence {
    fn encode_wire(&self, out: &mut WireWriter) {
        self.hop_index.encode_wire(out);
        self.relay_node_id.encode_wire(out);
        self.from.encode_wire(out);
        self.to.encode_wire(out);
        self.channel_id.encode_wire(out);
        self.route_hash.encode_wire(out);
        self.input_hash.encode_wire(out);
        self.packet_hash.encode_wire(out);
        self.sequence.encode_wire(out);
        self.previous_transit_hash.encode_wire(out);
        self.transit_hash.encode_wire(out);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        Ok(Self {
            hop_index: <u64 as EdgeWire>::decode_wire(input)?,
            relay_node_id: <NodeId as EdgeWire>::decode_wire(input)?,
            from: <NodeId as EdgeWire>::decode_wire(input)?,
            to: <NodeId as EdgeWire>::decode_wire(input)?,
            channel_id: <Hash as EdgeWire>::decode_wire(input)?,
            route_hash: <Hash as EdgeWire>::decode_wire(input)?,
            input_hash: <Hash as EdgeWire>::decode_wire(input)?,
            packet_hash: <Hash as EdgeWire>::decode_wire(input)?,
            sequence: <u64 as EdgeWire>::decode_wire(input)?,
            previous_transit_hash: <Hash as EdgeWire>::decode_wire(input)?,
            transit_hash: <Hash as EdgeWire>::decode_wire(input)?,
        })
    }
}

impl EdgeWire for RelayTransitBundle {
    fn encode_wire(&self, out: &mut WireWriter) {
        self.abi_version.encode_wire(out);
        self.request_hash.encode_wire(out);
        self.admission_hash.encode_wire(out);
        self.controlling_admission_node_id.encode_wire(out);
        self.source_node_id.encode_wire(out);
        self.destination_node_id.encode_wire(out);
        self.relay_path.encode_wire(out);
        self.packet_hash.encode_wire(out);
        self.final_delivery_proof_hash.encode_wire(out);
        self.hops.encode_wire(out);
        self.bundle_root.encode_wire(out);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        Ok(Self {
            abi_version: <u16 as EdgeWire>::decode_wire(input)?,
            request_hash: <Hash as EdgeWire>::decode_wire(input)?,
            admission_hash: <Hash as EdgeWire>::decode_wire(input)?,
            controlling_admission_node_id: <NodeId as EdgeWire>::decode_wire(input)?,
            source_node_id: <NodeId as EdgeWire>::decode_wire(input)?,
            destination_node_id: <NodeId as EdgeWire>::decode_wire(input)?,
            relay_path: <Vec<NodeId> as EdgeWire>::decode_wire(input)?,
            packet_hash: <Hash as EdgeWire>::decode_wire(input)?,
            final_delivery_proof_hash: <Hash as EdgeWire>::decode_wire(input)?,
            hops: <Vec<RelayTransitHopEvidence> as EdgeWire>::decode_wire(input)?,
            bundle_root: <Hash as EdgeWire>::decode_wire(input)?,
        })
    }
}

impl EdgeWire for RelayProofCustodyAck {
    fn encode_wire(&self, out: &mut WireWriter) {
        self.abi_version.encode_wire(out);
        self.custodian.encode_wire(out);
        self.request_hash.encode_wire(out);
        self.admission_hash.encode_wire(out);
        self.bundle_root.encode_wire(out);
        self.packed_proof_root.encode_wire(out);
        self.custody_kind.encode_wire(out);
        self.sequence_start.encode_wire(out);
        self.sequence_end.encode_wire(out);
        self.valid_until_unix_ms.encode_wire(out);
        self.signature.encode_wire(out);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        Ok(Self {
            abi_version: <u16 as EdgeWire>::decode_wire(input)?,
            custodian: <NodeIdentity as EdgeWire>::decode_wire(input)?,
            request_hash: <Hash as EdgeWire>::decode_wire(input)?,
            admission_hash: <Hash as EdgeWire>::decode_wire(input)?,
            bundle_root: <Hash as EdgeWire>::decode_wire(input)?,
            packed_proof_root: <Hash as EdgeWire>::decode_wire(input)?,
            custody_kind: <u16 as EdgeWire>::decode_wire(input)?,
            sequence_start: <u64 as EdgeWire>::decode_wire(input)?,
            sequence_end: <u64 as EdgeWire>::decode_wire(input)?,
            valid_until_unix_ms: <u64 as EdgeWire>::decode_wire(input)?,
            signature: <WorkSignature as EdgeWire>::decode_wire(input)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::{wire_bytes, wire_from_bytes};
    use crate::identity::node_identity_from_key;
    use crate::protocol::NODE_ROLE_RELAY;
    use crate::signing::empty_signature;
    use alloc::vec;
    use edgerun_crypto::Ed25519SigningKey;

    fn context<'a>(relay_path: &'a [NodeId]) -> RelayTransitBundleContext<'a> {
        RelayTransitBundleContext {
            request_hash: [1u8; 32],
            admission_hash: [2u8; 32],
            controlling_admission_node_id: [3u8; 32],
            source_node_id: [10u8; 32],
            destination_node_id: [99u8; 32],
            relay_path,
            packet_hash: [7u8; 32],
            final_delivery_proof_hash: [8u8; 32],
            max_hops: MAX_RELAY_TRANSIT_BUNDLE_HOPS,
        }
    }

    fn bundle() -> RelayTransitBundle {
        let relay_path = vec![[21u8; 32], [22u8; 32], [23u8; 32]];
        let ctx = context(&relay_path);
        let request_hash = ctx.request_hash;
        let admission_hash = ctx.admission_hash;
        let controlling_admission_node_id = ctx.controlling_admission_node_id;
        let source_node_id = ctx.source_node_id;
        let destination_node_id = ctx.destination_node_id;
        let packet_hash = ctx.packet_hash;
        let final_delivery_proof_hash = ctx.final_delivery_proof_hash;
        let hops = vec![
            relay_transit_hop_evidence(
                PacketTransitHashInput {
                    node_id: relay_path[0],
                    from: source_node_id,
                    to: relay_path[1],
                    channel_id: [31u8; 32],
                    route_hash: [41u8; 32],
                    packet_hash,
                    sequence: 4,
                    previous_transit_hash: [0u8; 32],
                },
                [51u8; 32],
                0,
            ),
            relay_transit_hop_evidence(
                PacketTransitHashInput {
                    node_id: relay_path[1],
                    from: relay_path[0],
                    to: relay_path[2],
                    channel_id: [32u8; 32],
                    route_hash: [42u8; 32],
                    packet_hash,
                    sequence: 7,
                    previous_transit_hash: [5u8; 32],
                },
                [52u8; 32],
                1,
            ),
            relay_transit_hop_evidence(
                PacketTransitHashInput {
                    node_id: relay_path[2],
                    from: relay_path[1],
                    to: destination_node_id,
                    channel_id: [33u8; 32],
                    route_hash: [43u8; 32],
                    packet_hash,
                    sequence: 2,
                    previous_transit_hash: [6u8; 32],
                },
                [53u8; 32],
                2,
            ),
        ];
        finalize_relay_transit_bundle(RelayTransitBundle {
            abi_version: WORK_WIRE_ABI_VERSION,
            request_hash,
            admission_hash,
            controlling_admission_node_id,
            source_node_id,
            destination_node_id,
            relay_path,
            packet_hash,
            final_delivery_proof_hash,
            hops,
            bundle_root: [0u8; 32],
        })
    }

    fn builder_for(relay_path: Vec<NodeId>) -> RelayTransitBundleBuilder {
        RelayTransitBundleBuilder::new(
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            [10u8; 32],
            [99u8; 32],
            relay_path,
            [7u8; 32],
            MAX_RELAY_TRANSIT_BUNDLE_HOPS,
        )
        .expect("builder")
    }

    fn push_fixture_hops(builder: &mut RelayTransitBundleBuilder, relay_path: &[NodeId]) {
        builder
            .push_hop(
                PacketTransitHashInput {
                    node_id: relay_path[0],
                    from: [10u8; 32],
                    to: relay_path[1],
                    channel_id: [31u8; 32],
                    route_hash: [41u8; 32],
                    packet_hash: [7u8; 32],
                    sequence: 4,
                    previous_transit_hash: [0u8; 32],
                },
                [51u8; 32],
            )
            .expect("first hop");
        builder
            .push_hop(
                PacketTransitHashInput {
                    node_id: relay_path[1],
                    from: relay_path[0],
                    to: relay_path[2],
                    channel_id: [32u8; 32],
                    route_hash: [42u8; 32],
                    packet_hash: [7u8; 32],
                    sequence: 7,
                    previous_transit_hash: [5u8; 32],
                },
                [52u8; 32],
            )
            .expect("second hop");
        builder
            .push_hop(
                PacketTransitHashInput {
                    node_id: relay_path[2],
                    from: relay_path[1],
                    to: [99u8; 32],
                    channel_id: [33u8; 32],
                    route_hash: [43u8; 32],
                    packet_hash: [7u8; 32],
                    sequence: 2,
                    previous_transit_hash: [6u8; 32],
                },
                [53u8; 32],
            )
            .expect("third hop");
    }

    #[test]
    fn relay_transit_bundle_verifies_multi_relay_path_and_wire_roundtrips() {
        let bundle = bundle();
        let ctx = context(&bundle.relay_path);
        let root = verify_relay_transit_bundle(&bundle, &ctx).expect("bundle verifies");
        assert_eq!(root, bundle.bundle_root);

        let bytes = wire_bytes(&bundle).expect("bundle wire bytes");
        let decoded = wire_from_bytes::<RelayTransitBundle, RelayTransitBundle>(&bytes)
            .expect("bundle wire roundtrip");
        assert_eq!(decoded, bundle);
    }

    #[test]
    fn relay_transit_bundle_builder_constructs_verifiable_bundle() {
        let relay_path = vec![[21u8; 32], [22u8; 32], [23u8; 32]];
        let mut builder = builder_for(relay_path.clone());
        push_fixture_hops(&mut builder, &relay_path);
        let route_commitment = builder.route_commitment();
        let bundle = builder
            .finish([8u8; 32])
            .expect("finished relay transit bundle");
        assert_eq!(
            route_commitment,
            relay_transit_bundle_route_commitment(&bundle)
        );
        let ctx = context(&bundle.relay_path);
        verify_relay_transit_bundle(&bundle, &ctx).expect("builder bundle verifies");
    }

    #[test]
    fn relay_transit_bundle_builder_rejects_wrong_order_packet_and_incomplete_path() {
        let relay_path = vec![[21u8; 32], [22u8; 32], [23u8; 32]];
        let mut wrong_order = builder_for(relay_path.clone());
        assert_eq!(
            wrong_order.push_hop(
                PacketTransitHashInput {
                    node_id: relay_path[1],
                    from: [10u8; 32],
                    to: relay_path[2],
                    channel_id: [32u8; 32],
                    route_hash: [42u8; 32],
                    packet_hash: [7u8; 32],
                    sequence: 7,
                    previous_transit_hash: [5u8; 32],
                },
                [52u8; 32],
            ),
            Err(RelayTransitBundleError::HopRelayMismatch)
        );

        let mut wrong_packet = builder_for(relay_path.clone());
        assert_eq!(
            wrong_packet.push_hop(
                PacketTransitHashInput {
                    node_id: relay_path[0],
                    from: [10u8; 32],
                    to: relay_path[1],
                    channel_id: [31u8; 32],
                    route_hash: [41u8; 32],
                    packet_hash: [77u8; 32],
                    sequence: 4,
                    previous_transit_hash: [0u8; 32],
                },
                [51u8; 32],
            ),
            Err(RelayTransitBundleError::PacketMismatch)
        );

        let mut incomplete = builder_for(relay_path.clone());
        incomplete
            .push_hop(
                PacketTransitHashInput {
                    node_id: relay_path[0],
                    from: [10u8; 32],
                    to: relay_path[1],
                    channel_id: [31u8; 32],
                    route_hash: [41u8; 32],
                    packet_hash: [7u8; 32],
                    sequence: 4,
                    previous_transit_hash: [0u8; 32],
                },
                [51u8; 32],
            )
            .expect("first hop");
        assert_eq!(
            incomplete.finish([8u8; 32]),
            Err(RelayTransitBundleError::PathMismatch)
        );
    }

    #[test]
    fn relay_transit_bundle_builder_rejects_missing_delivery_proof_and_oversized_path() {
        let relay_path = vec![[21u8; 32], [22u8; 32], [23u8; 32]];
        let mut builder = builder_for(relay_path.clone());
        push_fixture_hops(&mut builder, &relay_path);
        assert_eq!(
            builder.finish([0u8; 32]),
            Err(RelayTransitBundleError::MissingFinalDeliveryProof)
        );

        assert_eq!(
            RelayTransitBundleBuilder::new(
                [1u8; 32],
                [2u8; 32],
                [3u8; 32],
                [10u8; 32],
                [99u8; 32],
                vec![[1u8; 32], [2u8; 32]],
                [7u8; 32],
                1,
            ),
            Err(RelayTransitBundleError::TooManyHops)
        );
    }

    fn custody_ack(custodian_role: u16, custody_kind: u16) -> RelayProofCustodyAck {
        let key = Ed25519SigningKey::from_bytes(&[61u8; 32]);
        let custodian = node_identity_from_key(&key, custodian_role);
        sign_relay_proof_custody_ack(
            &key,
            RelayProofCustodyAck {
                abi_version: WORK_WIRE_ABI_VERSION,
                custodian,
                request_hash: [1u8; 32],
                admission_hash: [2u8; 32],
                bundle_root: [3u8; 32],
                packed_proof_root: [4u8; 32],
                custody_kind,
                sequence_start: 1,
                sequence_end: 3,
                valid_until_unix_ms: 10_000,
                signature: empty_signature(),
            },
        )
    }

    fn custody_ack_for_bundle(
        bundle: &RelayTransitBundle,
        sequence_start: u64,
        sequence_end: u64,
        valid_until_unix_ms: u64,
    ) -> RelayProofCustodyAck {
        let key = Ed25519SigningKey::from_bytes(&[63u8; 32]);
        let custodian = node_identity_from_key(&key, NODE_ROLE_STORAGE);
        sign_relay_proof_custody_ack(
            &key,
            RelayProofCustodyAck {
                abi_version: WORK_WIRE_ABI_VERSION,
                custodian,
                request_hash: bundle.request_hash,
                admission_hash: bundle.admission_hash,
                bundle_root: bundle.bundle_root,
                packed_proof_root: [64u8; 32],
                custody_kind: RELAY_PROOF_CUSTODY_KIND_STORED,
                sequence_start,
                sequence_end,
                valid_until_unix_ms,
                signature: empty_signature(),
            },
        )
    }

    fn canonical_custody_ack_for_bundle(bundle: &RelayTransitBundle) -> RelayProofCustodyAck {
        let key = Ed25519SigningKey::from_bytes(&[64u8; 32]);
        let custodian = node_identity_from_key(&key, NODE_ROLE_STORAGE);
        sign_relay_proof_custody_ack_for_bundle(
            &key,
            custodian,
            bundle,
            [66u8; 32],
            RELAY_PROOF_CUSTODY_KIND_STORED,
            10,
        )
        .expect("canonical custody ack")
    }

    fn canonical_custody_ack_for_kind(
        bundle: &RelayTransitBundle,
        seed: u8,
        role: u16,
        custody_kind: u16,
        packed_proof_root: Hash,
    ) -> RelayProofCustodyAck {
        let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
        let custodian = node_identity_from_key(&key, role);
        sign_relay_proof_custody_ack_for_bundle(
            &key,
            custodian,
            bundle,
            packed_proof_root,
            custody_kind,
            10,
        )
        .expect("custody ack for kind")
    }

    #[test]
    fn relay_proof_custody_ack_is_signed_and_wire_roundtrips() {
        let ack = custody_ack(NODE_ROLE_STORAGE, RELAY_PROOF_CUSTODY_KIND_PACKED);
        let hash = verify_relay_proof_custody_ack(&ack).expect("custody ack verifies");
        assert_eq!(hash, relay_proof_custody_ack_hash(&ack));

        let bytes = wire_bytes(&ack).expect("ack wire bytes");
        let decoded = wire_from_bytes::<RelayProofCustodyAck, RelayProofCustodyAck>(&bytes)
            .expect("ack roundtrip");
        assert_eq!(decoded, ack);
    }

    #[test]
    fn relay_proof_custody_ack_rejects_wrong_role_kind_range_and_tamper() {
        let wrong_role = custody_ack(NODE_ROLE_RELAY, RELAY_PROOF_CUSTODY_KIND_PACKED);
        assert_eq!(
            verify_relay_proof_custody_ack(&wrong_role),
            Err(RelayProofCustodyAckError::WrongCustodianRole)
        );

        let bad_kind = custody_ack(NODE_ROLE_STORAGE, 99);
        assert_eq!(
            verify_relay_proof_custody_ack(&bad_kind),
            Err(RelayProofCustodyAckError::InvalidCustodyKind)
        );

        let wrong_kind_role = custody_ack(NODE_ROLE_STORAGE, RELAY_PROOF_CUSTODY_KIND_NOTARIZED);
        assert_eq!(
            verify_relay_proof_custody_ack(&wrong_kind_role),
            Err(RelayProofCustodyAckError::CustodyKindRoleMismatch)
        );

        let wrong_stored_role = custody_ack(NODE_ROLE_NOTARY, RELAY_PROOF_CUSTODY_KIND_STORED);
        assert_eq!(
            verify_relay_proof_custody_ack(&wrong_stored_role),
            Err(RelayProofCustodyAckError::CustodyKindRoleMismatch)
        );

        let mut empty_root = custody_ack(NODE_ROLE_STORAGE, RELAY_PROOF_CUSTODY_KIND_STORED);
        empty_root.packed_proof_root = [0u8; 32];
        assert_eq!(
            verify_relay_proof_custody_ack(&empty_root),
            Err(RelayProofCustodyAckError::EmptyPackedProofRoot)
        );

        let key = Ed25519SigningKey::from_bytes(&[62u8; 32]);
        let mut bad_range = sign_relay_proof_custody_ack(
            &key,
            RelayProofCustodyAck {
                abi_version: WORK_WIRE_ABI_VERSION,
                custodian: node_identity_from_key(&key, NODE_ROLE_NOTARY),
                request_hash: [1u8; 32],
                admission_hash: [2u8; 32],
                bundle_root: [3u8; 32],
                packed_proof_root: [4u8; 32],
                custody_kind: RELAY_PROOF_CUSTODY_KIND_NOTARIZED,
                sequence_start: 4,
                sequence_end: 3,
                valid_until_unix_ms: 10_000,
                signature: empty_signature(),
            },
        );
        assert_eq!(
            verify_relay_proof_custody_ack(&bad_range),
            Err(RelayProofCustodyAckError::InvalidRange)
        );

        bad_range.sequence_start = 1;
        assert_eq!(
            verify_relay_proof_custody_ack(&bad_range),
            Err(RelayProofCustodyAckError::InvalidSignature)
        );
    }

    #[test]
    fn relay_proof_custody_ack_constructor_binds_bundle_window_and_root() {
        let bundle = bundle();
        let ack = canonical_custody_ack_for_bundle(&bundle);
        assert_eq!(ack.sequence_start, 1);
        assert_eq!(ack.sequence_end, bundle.hops.len() as u64);
        assert_eq!(ack.request_hash, bundle.request_hash);
        assert_eq!(ack.admission_hash, bundle.admission_hash);
        assert_eq!(ack.bundle_root, bundle.bundle_root);

        let hash = verify_relay_proof_custody_ack_for_bundle(&ack, &bundle, 9)
            .expect("constructor output verifies");
        assert_eq!(hash, relay_proof_custody_ack_hash(&ack));
    }

    #[test]
    fn relay_proof_custody_ack_constructor_rejects_bad_inputs() {
        let bundle = bundle();
        let key = Ed25519SigningKey::from_bytes(&[65u8; 32]);
        let wrong_role = node_identity_from_key(&key, NODE_ROLE_RELAY);
        assert_eq!(
            sign_relay_proof_custody_ack_for_bundle(
                &key,
                wrong_role,
                &bundle,
                [66u8; 32],
                RELAY_PROOF_CUSTODY_KIND_STORED,
                10,
            ),
            Err(RelayProofCustodyAckError::WrongCustodianRole)
        );

        let storage = node_identity_from_key(&key, NODE_ROLE_STORAGE);
        assert_eq!(
            sign_relay_proof_custody_ack_for_bundle(
                &key,
                storage.clone(),
                &bundle,
                [0u8; 32],
                RELAY_PROOF_CUSTODY_KIND_STORED,
                10,
            ),
            Err(RelayProofCustodyAckError::EmptyPackedProofRoot)
        );

        let mut tampered_bundle = bundle.clone();
        tampered_bundle.bundle_root = [67u8; 32];
        assert_eq!(
            sign_relay_proof_custody_ack_for_bundle(
                &key,
                storage,
                &tampered_bundle,
                [66u8; 32],
                RELAY_PROOF_CUSTODY_KIND_STORED,
                10,
            ),
            Err(RelayProofCustodyAckError::BundleMismatch)
        );
    }

    #[test]
    fn relay_proof_custody_chain_verifies_progression_to_required_kind() {
        let bundle = bundle();
        let packed_root = [70u8; 32];
        let packed = canonical_custody_ack_for_kind(
            &bundle,
            70,
            NODE_ROLE_STORAGE,
            RELAY_PROOF_CUSTODY_KIND_PACKED,
            packed_root,
        );
        let notarized = canonical_custody_ack_for_kind(
            &bundle,
            71,
            NODE_ROLE_NOTARY,
            RELAY_PROOF_CUSTODY_KIND_NOTARIZED,
            packed_root,
        );
        let stored = canonical_custody_ack_for_kind(
            &bundle,
            72,
            NODE_ROLE_STORAGE,
            RELAY_PROOF_CUSTODY_KIND_STORED,
            packed_root,
        );
        let chain = vec![packed, notarized, stored.clone()];

        let hash = verify_relay_proof_custody_chain(
            &bundle,
            &chain,
            packed_root,
            RELAY_PROOF_CUSTODY_KIND_STORED,
            9,
        )
        .expect("custody chain verifies");
        assert_eq!(hash, relay_proof_custody_ack_hash(&stored));
    }

    #[test]
    fn relay_proof_custody_chain_rejects_wrong_root_downgrade_and_final_kind() {
        let bundle = bundle();
        let packed_root = [73u8; 32];
        let packed = canonical_custody_ack_for_kind(
            &bundle,
            73,
            NODE_ROLE_VERIFIER,
            RELAY_PROOF_CUSTODY_KIND_PACKED,
            packed_root,
        );
        let notarized = canonical_custody_ack_for_kind(
            &bundle,
            74,
            NODE_ROLE_NOTARY,
            RELAY_PROOF_CUSTODY_KIND_NOTARIZED,
            packed_root,
        );
        let stored = canonical_custody_ack_for_kind(
            &bundle,
            75,
            NODE_ROLE_STORAGE,
            RELAY_PROOF_CUSTODY_KIND_STORED,
            packed_root,
        );

        assert_eq!(
            verify_relay_proof_custody_chain(
                &bundle,
                &[],
                packed_root,
                RELAY_PROOF_CUSTODY_KIND_STORED,
                9,
            ),
            Err(RelayProofCustodyAckError::EmptyChain)
        );
        assert_eq!(
            verify_relay_proof_custody_chain(
                &bundle,
                &[packed.clone(), notarized.clone(), stored.clone()],
                [74u8; 32],
                RELAY_PROOF_CUSTODY_KIND_STORED,
                9,
            ),
            Err(RelayProofCustodyAckError::CustodyChainMismatch)
        );
        assert_eq!(
            verify_relay_proof_custody_chain(
                &bundle,
                &[notarized.clone(), packed.clone(), stored.clone()],
                packed_root,
                RELAY_PROOF_CUSTODY_KIND_STORED,
                9,
            ),
            Err(RelayProofCustodyAckError::CustodyChainDowngrade)
        );
        assert_eq!(
            verify_relay_proof_custody_chain(
                &bundle,
                &[packed.clone(), stored.clone()],
                packed_root,
                RELAY_PROOF_CUSTODY_KIND_STORED,
                9,
            ),
            Err(RelayProofCustodyAckError::CustodyChainDowngrade)
        );
        assert_eq!(
            verify_relay_proof_custody_chain(
                &bundle,
                &[packed, notarized],
                packed_root,
                RELAY_PROOF_CUSTODY_KIND_STORED,
                9,
            ),
            Err(RelayProofCustodyAckError::CustodyChainMismatch)
        );
    }

    #[test]
    fn relay_proof_custody_ack_verifies_against_bundle_context() {
        let bundle = bundle();
        let ack = custody_ack_for_bundle(&bundle, 1, bundle.hops.len() as u64, 10);
        let hash = verify_relay_proof_custody_ack_for_bundle(&ack, &bundle, 9)
            .expect("ack verifies against bundle");
        assert_eq!(hash, relay_proof_custody_ack_hash(&ack));
    }

    #[test]
    fn relay_proof_custody_ack_rejects_replay_wrong_window_and_expiry() {
        let bundle = bundle();
        let ack = custody_ack_for_bundle(&bundle, 1, bundle.hops.len() as u64, 10);

        let mut other_bundle = bundle.clone();
        other_bundle.packet_hash = [65u8; 32];
        other_bundle.bundle_root = relay_transit_bundle_root(&other_bundle);
        assert_eq!(
            verify_relay_proof_custody_ack_for_bundle(&ack, &other_bundle, 9),
            Err(RelayProofCustodyAckError::BundleMismatch)
        );

        let wrong_window = custody_ack_for_bundle(&bundle, 1, 99, 10);
        assert_eq!(
            verify_relay_proof_custody_ack_for_bundle(&wrong_window, &bundle, 9),
            Err(RelayProofCustodyAckError::SequenceWindowMismatch)
        );

        assert_eq!(
            verify_relay_proof_custody_ack_for_bundle(&ack, &bundle, 11),
            Err(RelayProofCustodyAckError::Expired)
        );
    }

    #[test]
    fn relay_transit_bundle_rejects_cross_domain_or_wrong_path_substitution() {
        let bundle = bundle();
        let mut wrong_domain = context(&bundle.relay_path);
        wrong_domain.admission_hash = [44u8; 32];
        assert_eq!(
            verify_relay_transit_bundle(&bundle, &wrong_domain),
            Err(RelayTransitBundleError::ContextMismatch)
        );

        let wrong_path = vec![[21u8; 32], [24u8; 32], [23u8; 32]];
        let wrong_path_ctx = context(&wrong_path);
        assert_eq!(
            verify_relay_transit_bundle(&bundle, &wrong_path_ctx),
            Err(RelayTransitBundleError::PathMismatch)
        );
    }

    #[test]
    fn relay_transit_bundle_rejects_tampered_hop_packet_hash_and_endpoints() {
        let bundle = bundle();
        let ctx = context(&bundle.relay_path);

        let mut tampered_packet = bundle.clone();
        tampered_packet.hops[1].packet_hash = [55u8; 32];
        tampered_packet.bundle_root = relay_transit_bundle_root(&tampered_packet);
        assert_eq!(
            verify_relay_transit_bundle(&tampered_packet, &ctx),
            Err(RelayTransitBundleError::PacketMismatch)
        );

        let mut tampered_endpoint = bundle.clone();
        tampered_endpoint.hops[2].from = [66u8; 32];
        tampered_endpoint.hops[2].transit_hash = packet_transit_hash(&PacketTransitHashInput {
            node_id: tampered_endpoint.hops[2].relay_node_id,
            from: tampered_endpoint.hops[2].from,
            to: tampered_endpoint.hops[2].to,
            channel_id: tampered_endpoint.hops[2].channel_id,
            route_hash: tampered_endpoint.hops[2].route_hash,
            packet_hash: tampered_endpoint.hops[2].packet_hash,
            sequence: tampered_endpoint.hops[2].sequence,
            previous_transit_hash: tampered_endpoint.hops[2].previous_transit_hash,
        });
        tampered_endpoint.bundle_root = relay_transit_bundle_root(&tampered_endpoint);
        assert_eq!(
            verify_relay_transit_bundle(&tampered_endpoint, &ctx),
            Err(RelayTransitBundleError::HopEndpointMismatch)
        );
    }

    #[test]
    fn relay_transit_bundle_rejects_summary_root_without_valid_hops() {
        let bundle = bundle();
        let ctx = context(&bundle.relay_path);
        let mut tampered = bundle.clone();
        tampered.hops[0].transit_hash = [77u8; 32];
        tampered.bundle_root = relay_transit_bundle_root(&tampered);
        assert_eq!(
            verify_relay_transit_bundle(&tampered, &ctx),
            Err(RelayTransitBundleError::TransitHashMismatch)
        );
    }

    #[test]
    fn relay_transit_bundle_rejects_empty_or_oversized_paths() {
        let mut empty = bundle();
        empty.relay_path.clear();
        empty.hops.clear();
        empty.bundle_root = relay_transit_bundle_root(&empty);
        let empty_ctx = context(&empty.relay_path);
        assert_eq!(
            verify_relay_transit_bundle(&empty, &empty_ctx),
            Err(RelayTransitBundleError::EmptyPath)
        );

        let oversized_path = vec![[9u8; 32]; 2];
        let ctx = RelayTransitBundleContext {
            max_hops: 1,
            ..context(&oversized_path)
        };
        let mut oversized = bundle();
        oversized.relay_path = oversized_path.clone();
        oversized.bundle_root = relay_transit_bundle_root(&oversized);
        assert_eq!(
            verify_relay_transit_bundle(&oversized, &ctx),
            Err(RelayTransitBundleError::TooManyHops)
        );
    }
}
