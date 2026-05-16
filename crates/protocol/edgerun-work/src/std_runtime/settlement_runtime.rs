use std::sync::{Arc, Mutex};

use crate::batch_settlement::{BatchSettlementError, BatchSettlementResult};
use crate::protocol::{Hash, NodeId, PublicKey, WorkAdmission, WorkReceipt};
use crate::settlement::{
    DeliverySettlementEvidence, RelayTransitCustodyChainEvidence, RelayTransitCustodyCheckResult,
    RelayTransitCustodyEvidence, RelayTransitCustodyRequirement,
    RelayTransitCustodySettlementResult, RelayTransitSettlementEvidence, SettlementError,
    SettlementLedger, SettlementPruneResult, SettlementResult,
    verify_relay_transit_custody_chain_evidence, verify_relay_transit_custody_evidence,
    verify_relay_transit_custody_requirement,
};

#[derive(Clone, Debug, Default)]
pub struct ThreadSafeSettlementLedger {
    inner: Arc<Mutex<SettlementLedger>>,
}

impl ThreadSafeSettlementLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_ledger(ledger: SettlementLedger) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ledger)),
        }
    }

    pub fn deposit_user_credit(&self, user: PublicKey, amount: u64) -> u64 {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .deposit_user_credit(user, amount)
    }

    pub fn user_balance(&self, user: &PublicKey) -> u64 {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .user_balance(user)
    }

    pub fn worker_balance(&self, worker: &NodeId) -> u64 {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .worker_balance(worker)
    }

    pub fn reserve_admission_budget(
        &self,
        admission: &WorkAdmission,
    ) -> Result<Hash, SettlementError> {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .reserve_admission_budget(admission)
    }

    pub fn settle_receipt_unchecked_evidence(
        &self,
        admission: &WorkAdmission,
        receipt: &WorkReceipt,
    ) -> Result<SettlementResult, SettlementError> {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .settle_receipt_unchecked_evidence(admission, receipt)
    }

    pub fn settle_receipt(
        &self,
        admission: &WorkAdmission,
        receipt: &WorkReceipt,
    ) -> Result<SettlementResult, SettlementError> {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .settle_receipt(admission, receipt)
    }

    pub fn settle_delivery(
        &self,
        evidence: &DeliverySettlementEvidence<'_>,
    ) -> Result<SettlementResult, SettlementError> {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .settle_delivery(evidence)
    }

    pub fn can_settle_relay_transit_hop(
        &self,
        evidence: &RelayTransitSettlementEvidence<'_>,
    ) -> Result<(), SettlementError> {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .can_settle_relay_transit_hop(evidence)
    }

    pub fn settle_relay_transit_hop(
        &self,
        evidence: &RelayTransitSettlementEvidence<'_>,
    ) -> Result<SettlementResult, SettlementError> {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .settle_relay_transit_hop(evidence)
    }

    pub fn can_settle_relay_transit_hop_with_custody(
        &self,
        transit: &RelayTransitSettlementEvidence<'_>,
        custody: &RelayTransitCustodyChainEvidence<'_>,
    ) -> Result<RelayTransitCustodyCheckResult, SettlementError> {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .can_settle_relay_transit_hop_with_custody(transit, custody)
    }

    pub fn settle_relay_transit_hop_with_custody(
        &self,
        transit: &RelayTransitSettlementEvidence<'_>,
        custody: &RelayTransitCustodyChainEvidence<'_>,
    ) -> Result<RelayTransitCustodySettlementResult, SettlementError> {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .settle_relay_transit_hop_with_custody(transit, custody)
    }

    pub fn verify_relay_transit_custody_requirement(
        &self,
        requirement: &RelayTransitCustodyRequirement,
    ) -> Result<(), SettlementError> {
        verify_relay_transit_custody_requirement(requirement)
    }

    pub fn verify_relay_transit_custody_evidence(
        &self,
        evidence: &RelayTransitCustodyEvidence<'_>,
    ) -> Result<Hash, SettlementError> {
        verify_relay_transit_custody_evidence(evidence)
    }

    pub fn verify_relay_transit_custody_chain_evidence(
        &self,
        evidence: &RelayTransitCustodyChainEvidence<'_>,
    ) -> Result<Hash, SettlementError> {
        verify_relay_transit_custody_chain_evidence(evidence)
    }

    pub fn settle_receipt_batch_unchecked_evidence(
        &self,
        admission: &WorkAdmission,
        receipts: &[WorkReceipt],
    ) -> Result<BatchSettlementResult, BatchSettlementError> {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .settle_receipt_batch_unchecked_evidence(admission, receipts)
    }

    pub fn settle_receipt_batch(
        &self,
        admission: &WorkAdmission,
        receipts: &[WorkReceipt],
    ) -> Result<BatchSettlementResult, BatchSettlementError> {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .settle_receipt_batch(admission, receipts)
    }

    pub fn prune_finalized_admission(&self, admission_hash: &Hash) -> SettlementPruneResult {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .prune_finalized_admission(admission_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::{CHANNEL_KIND_MEMORY, ChannelEndpoint, ChannelEnvelope};
    use crate::channel_order::OrderedChannelEnvelope;
    use crate::codec::packet_bytes;
    use crate::delivery_proof::{channel_proof_for_allowed_ordered_message, channel_proof_hash};
    use crate::protocol::*;
    use crate::recipient_policy::{
        open_recipient_message_policy, recipient_message_policy_hash, sign_recipient_message_policy,
    };
    use crate::signing::{empty_signature, sign_work_admission, sign_work_receipt};
    use crate::transit_proof::{
        PacketTransitHashInput, RELAY_PROOF_CUSTODY_KIND_NOTARIZED,
        RELAY_PROOF_CUSTODY_KIND_PACKED, RELAY_PROOF_CUSTODY_KIND_STORED, RelayTransitBundle,
        finalize_relay_transit_bundle, relay_transit_hop_evidence, relay_transit_route_commitment,
        sign_relay_proof_custody_ack_for_bundle,
    };
    use crate::{
        SimNode, blake3_hash, receipt_id_for_claim, sign_network_message_payload,
        work_admission_hash,
    };
    use alloc::vec;
    use alloc::vec::Vec;
    use edgerun_crypto::Ed25519SigningKey;

    fn public_key_from_seed(seed: u8) -> PublicKey {
        let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
        let mut out = [0u8; 32];
        out.copy_from_slice(key.verifying_key().as_bytes());
        out
    }

    #[test]
    fn thread_safe_wrapper_settles_admitted_multi_relay_hop() {
        let admission_node = SimNode::from_seed(201, NODE_ROLE_ADMISSION);
        let user = public_key_from_seed(202);
        let relays = [
            SimNode::from_seed(203, NODE_ROLE_RELAY),
            SimNode::from_seed(204, NODE_ROLE_RELAY),
            SimNode::from_seed(205, NODE_ROLE_RELAY),
        ];
        let relay_path = relays
            .iter()
            .map(|relay| relay.identity.node_id)
            .collect::<Vec<_>>();
        let request_hash = blake3_hash(b"runtime-wrapper-relay-transit-request");
        let source_node = SimNode::from_seed(206, NODE_ROLE_MESSAGE);
        let recipient = SimNode::from_seed(207, NODE_ROLE_MESSAGE);
        let source = source_node.identity.node_id;
        let destination = recipient.identity.node_id;
        let packet = WorkPacket::NetworkMessage(sign_network_message_payload(
            &source_node.key,
            blake3_hash(b"runtime-wrapper-relay-transit-message"),
            [0u8; 32],
            source,
            destination,
            relay_path[2],
            DEPARTMENT_MESSAGE,
            WORK_TYPE_MESSAGE_DELIVER,
            1,
            b"runtime wrapper multi-hop relay delivery".to_vec(),
        ));
        let packet_hash = packet_bytes(&packet)
            .map(|bytes| blake3_hash(&bytes))
            .expect("packet hash");
        let mut recipient_policy =
            open_recipient_message_policy(recipient.identity.clone(), 1, u64::MAX);
        recipient_policy.allowed_relays.push(relay_path[2]);
        recipient_policy = sign_recipient_message_policy(&recipient.key, recipient_policy);
        let recipient_delivery = OrderedChannelEnvelope {
            envelope: ChannelEnvelope::new(
                [33u8; 32],
                relay_path[2],
                destination,
                [43u8; 32],
                packet_hash,
                packet,
            ),
            sequence: 1,
            previous_message_hash: [0u8; 32],
        };
        let recipient_proof = channel_proof_for_allowed_ordered_message(
            &recipient.key,
            &recipient.identity,
            relay_path[2],
            &recipient_delivery,
            &recipient_policy,
            1,
        )
        .expect("recipient proof");
        let hops = vec![
            relay_transit_hop_evidence(
                PacketTransitHashInput {
                    node_id: relay_path[0],
                    from: source,
                    to: relay_path[1],
                    channel_id: [31u8; 32],
                    route_hash: [41u8; 32],
                    packet_hash,
                    sequence: 1,
                    previous_transit_hash: [0u8; 32],
                },
                blake3_hash(b"runtime-wrapper-relay-hop-0-input"),
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
                    sequence: 1,
                    previous_transit_hash: [5u8; 32],
                },
                blake3_hash(b"runtime-wrapper-relay-hop-1-input"),
                1,
            ),
            relay_transit_hop_evidence(
                PacketTransitHashInput {
                    node_id: relay_path[2],
                    from: relay_path[1],
                    to: destination,
                    channel_id: [33u8; 32],
                    route_hash: [43u8; 32],
                    packet_hash,
                    sequence: 1,
                    previous_transit_hash: [6u8; 32],
                },
                blake3_hash(b"runtime-wrapper-relay-hop-2-input"),
                2,
            ),
        ];
        let route_commitment =
            relay_transit_route_commitment(source, destination, &relay_path, &hops);
        let admission = sign_work_admission(
            &admission_node.key,
            WorkAdmission {
                abi_version: WORK_WIRE_ABI_VERSION,
                admission_id: blake3_hash(b"runtime-wrapper-relay-transit-admission"),
                dao_id: admission_node.identity.public_key,
                user,
                admission_node: admission_node.identity.clone(),
                request_hash,
                assigned_route_commitment: route_commitment,
                assigned_channel: ChannelEndpoint::new(
                    blake3_hash(b"runtime-wrapper-relay-transit-channel"),
                    CHANNEL_KIND_MEMORY,
                    Vec::new(),
                    "runtime-wrapper-relay-transit".into(),
                ),
                assigned_relay_path: relay_path.clone(),
                admitted_budget: 30,
                policy_hash: recipient_message_policy_hash(&recipient_policy),
                sequence: 1,
                valid_until_unix_ms: u64::MAX,
                signature: empty_signature(),
            },
        );
        let admission_hash = work_admission_hash(&admission).expect("admission hash");
        let bundle = finalize_relay_transit_bundle(RelayTransitBundle {
            abi_version: WORK_WIRE_ABI_VERSION,
            request_hash,
            admission_hash,
            controlling_admission_node_id: admission.admission_node.node_id,
            source_node_id: source,
            destination_node_id: destination,
            relay_path,
            packet_hash,
            final_delivery_proof_hash: channel_proof_hash(&recipient_proof),
            hops,
            bundle_root: [0u8; 32],
        });
        let hop = &bundle.hops[1];
        let receipt = sign_work_receipt(
            &relays[1].key,
            WorkReceipt {
                abi_version: WORK_WIRE_ABI_VERSION,
                receipt_id: receipt_id_for_claim(
                    request_hash,
                    admission_hash,
                    relays[1].identity.node_id,
                    hop.input_hash,
                    hop.transit_hash,
                    hop.sequence,
                ),
                request_hash,
                admission_hash,
                worker: relays[1].identity.clone(),
                relay_node_id: relays[1].identity.node_id,
                input_hash: hop.input_hash,
                output_hash: hop.transit_hash,
                units_used: 1,
                total_claim: 7,
                sequence: hop.sequence,
                signature: empty_signature(),
            },
        );
        let evidence = RelayTransitSettlementEvidence {
            admission: &admission,
            receipt: &receipt,
            bundle: &bundle,
            recipient_delivery: &recipient_delivery,
            recipient: &recipient.identity,
            recipient_policy: &recipient_policy,
            recipient_proof: &recipient_proof,
            source_node_id: source,
            destination_node_id: destination,
            now_unix_ms: 1,
            max_hops: 8,
        };
        let ledger = ThreadSafeSettlementLedger::new();
        ledger.deposit_user_credit(user, 30);
        ledger
            .reserve_admission_budget(&admission)
            .expect("reserve relay admission budget");

        let packed_proof_root = blake3_hash(b"runtime-wrapper-relay-packed-proof-root");
        let verifier = SimNode::from_seed(208, NODE_ROLE_VERIFIER);
        let notary = SimNode::from_seed(209, NODE_ROLE_NOTARY);
        let storage = SimNode::from_seed(210, NODE_ROLE_STORAGE);
        let packed_ack = sign_relay_proof_custody_ack_for_bundle(
            &verifier.key,
            verifier.identity,
            &bundle,
            packed_proof_root,
            RELAY_PROOF_CUSTODY_KIND_PACKED,
            10,
        )
        .expect("packed custody ack");
        let notarized_ack = sign_relay_proof_custody_ack_for_bundle(
            &notary.key,
            notary.identity,
            &bundle,
            packed_proof_root,
            RELAY_PROOF_CUSTODY_KIND_NOTARIZED,
            10,
        )
        .expect("notarized custody ack");
        let stored_ack = sign_relay_proof_custody_ack_for_bundle(
            &storage.key,
            storage.identity,
            &bundle,
            packed_proof_root,
            RELAY_PROOF_CUSTODY_KIND_STORED,
            10,
        )
        .expect("stored custody ack");
        let stored_requirement = RelayTransitCustodyRequirement {
            expected_packed_proof_root: packed_proof_root,
            required_custody_kind: RELAY_PROOF_CUSTODY_KIND_STORED,
        };
        let custody_chain = [packed_ack, notarized_ack, stored_ack.clone()];
        let custody = RelayTransitCustodyChainEvidence {
            bundle: &bundle,
            custody_acks: &custody_chain,
            requirement: stored_requirement,
            now_unix_ms: 9,
        };

        ledger
            .verify_relay_transit_custody_evidence(&RelayTransitCustodyEvidence {
                bundle: &bundle,
                custody_ack: &stored_ack,
                requirement: stored_requirement,
                now_unix_ms: 9,
            })
            .expect("stored custody ack verifies through runtime wrapper");
        ledger
            .verify_relay_transit_custody_chain_evidence(&custody)
            .expect("custody chain verifies through runtime wrapper");
        let check = ledger
            .can_settle_relay_transit_hop_with_custody(&evidence, &custody)
            .expect("relay hop can settle with custody chain");
        assert_eq!(
            check.custody_hash,
            crate::transit_proof::relay_proof_custody_ack_hash(&stored_ack)
        );
        assert_eq!(
            check.custody_requirement_hash,
            crate::settlement::relay_transit_custody_requirement_hash(&stored_requirement)
        );
        let result = ledger
            .settle_relay_transit_hop_with_custody(&evidence, &custody)
            .expect("relay hop settles with custody through runtime wrapper");

        assert_eq!(result.settlement.amount, 7);
        assert_eq!(result.custody, check);
        assert_eq!(
            result.custody.custody_hash,
            crate::transit_proof::relay_proof_custody_ack_hash(&stored_ack)
        );
        assert_eq!(
            result.custody.custody_requirement_hash,
            crate::settlement::relay_transit_custody_requirement_hash(&stored_requirement)
        );
        assert_eq!(ledger.worker_balance(&relays[1].identity.node_id), 7);
    }
}
