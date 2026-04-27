//! Core protocol types — re-exported from protobuf as the canonical types.
//!
//! Every domain type is a protobuf-generated struct with `prost::Message`.
//! Canonical encoding = `prost::Message::encode()` directly.
//! No intermediate encoding layer, no conversion, no CBOR.

use crate::prelude::v1::*;

// Timestamp / Duration from prost-types (protobuf standard)
pub use prost_types::{Duration, Timestamp};

// Re-export proto types as the canonical protocol types.
pub use edgerun_proto::edgerun::v0::{
    access::{
        CostLimit, FederatedAggregateDescriptor, QueryRequest, QueryResultFragment,
        SnapshotDescriptor,
    },
    common::{
        CommandRef, DelegationRef, Digest, EventRef, HeadRef, IdentityRef, NodeRef, ObjectRef,
        RateLimit, RepresentationRef, RevocationRef, Signature, SnapshotRef, StreamRef, TimeWindow,
    },
    identity::IdentityRecord,
    network::{ReachabilityHint, RelayEnvelope, RouteAdvertisement, SessionAccept, SessionHello},
    object::{ChunkEntry, ChunkManifest, LogicalObjectDescriptor, StoredRepresentationHeader},
    stream::{
        ActionLifecyclePayload, CollectionCreatedPayload, CollectionDeletedPayload,
        CommandEnvelope, CommandResultPayload, CommandSentPayload, EventEnvelope,
        NodeGenesisPayload, SecretDeletePayload, SecretPutPayload,
    },
    trust::{
        AggregateTrustPolicy, AssuranceClaim, AssuranceRequirement, CapabilityDescriptor,
        ConstraintSet, DelegationRecord, RevocationRecord, RouteSelectionPolicy,
        RouteTrustAssignment, RouteTrustAssignments, ScopeDescriptor,
    },
};

// Proof types
pub use edgerun_proto::edgerun::v0::access::{
    AggregateSummaryProof, EventSetProof, ObjectAssertionProof, ProofBundle, ResultFragmentProof,
    SnapshotSetProof, StreamHeadsProof, TrustPolicyProof,
};

// CheckpointRef
pub use edgerun_proto::edgerun::v0::common::CheckpointRef;

// Enum types from proto (needed for event/command type fields)
pub use edgerun_proto::edgerun::v0::stream::{
    ActionStatus, CommandDecision, CommandType, EventType,
};

// ---------------------------------------------------------------------------
// Unified ProtocolRecord enum — wraps all canonical types for hashing/signing
// ---------------------------------------------------------------------------

/// A protocol record that can be canonicalized, hashed, or signed.
///
/// Each variant holds a protobuf-generated type that already implements
/// `prost::Message`, so canonical bytes = `prost::Message::encode()`.
#[derive(Clone, Debug)]
pub enum ProtocolRecord {
    IdentityRef(IdentityRef),
    NodeRef(NodeRef),
    StreamRef(StreamRef),
    EventRef(EventRef),
    HeadRef(HeadRef),
    CheckpointRef(CheckpointRef),
    ObjectRef(ObjectRef),
    RepresentationRef(RepresentationRef),
    CommandRef(CommandRef),
    DelegationRef(DelegationRef),
    RevocationRef(RevocationRef),
    SnapshotRef(SnapshotRef),
    Digest(Digest),
    Signature(Signature),
    TimeWindow(TimeWindow),
    CostLimit(CostLimit),
    ScopeDescriptor(ScopeDescriptor),
    DelegationRecord(DelegationRecord),
    CommandEnvelope(CommandEnvelope),
    EventEnvelope(EventEnvelope),
    SnapshotDescriptor(SnapshotDescriptor),
    QueryRequest(QueryRequest),
    QueryResultFragment(QueryResultFragment),
    FederatedAggregateDescriptor(FederatedAggregateDescriptor),
    IdentityRecord(IdentityRecord),
    AssuranceClaim(AssuranceClaim),
    RevocationRecord(RevocationRecord),
    RelayEnvelope(RelayEnvelope),
    ReachabilityHint(ReachabilityHint),
    RouteAdvertisement(RouteAdvertisement),
    SessionHello(SessionHello),
    SessionAccept(SessionAccept),
    NodeGenesisPayload(NodeGenesisPayload),
    CommandSentPayload(CommandSentPayload),
    CommandResultPayload(CommandResultPayload),
    ActionLifecyclePayload(ActionLifecyclePayload),
    SecretPutPayload(SecretPutPayload),
    SecretDeletePayload(SecretDeletePayload),
    CollectionCreatedPayload(CollectionCreatedPayload),
    CollectionDeletedPayload(CollectionDeletedPayload),
    LogicalObjectDescriptor(LogicalObjectDescriptor),
    StoredRepresentationHeader(StoredRepresentationHeader),
    ChunkEntry(ChunkEntry),
    ChunkManifest(ChunkManifest),
    AggregateTrustPolicy(AggregateTrustPolicy),
    RouteTrustAssignment(RouteTrustAssignment),
    RouteTrustAssignments(RouteTrustAssignments),
    RouteSelectionPolicy(RouteSelectionPolicy),
    AssuranceRequirement(AssuranceRequirement),
    ConstraintSet(ConstraintSet),
    CapabilityDescriptor(CapabilityDescriptor),
    StreamHeadsProof(StreamHeadsProof),
    SnapshotSetProof(SnapshotSetProof),
    EventSetProof(EventSetProof),
    ObjectAssertionProof(ObjectAssertionProof),
    ResultFragmentProof(ResultFragmentProof),
    AggregateSummaryProof(AggregateSummaryProof),
    TrustPolicyProof(TrustPolicyProof),
    ProofBundle(ProofBundle),
}

// ---------------------------------------------------------------------------
// Canonical encoding — signable form clears the signature field
// ---------------------------------------------------------------------------

/// Returns canonical protobuf wire bytes for a protocol record.
/// When `signable` is `true`, the signature field is cleared before encoding
/// so that the same bytes are produced regardless of which signature is attached.
pub fn canonical_bytes(record: &ProtocolRecord, signable: bool) -> Vec<u8> {
    macro_rules! encode_all {
        (
            signable: [$($signable_variant:ident),+ $(,)?],
            plain: [$($plain_variant:ident),+ $(,)?],
        ) => {{
            match record {
                // Signable variants (have a `.signature` field)
                $(
                    ProtocolRecord::$signable_variant(v) => {
                        let mut buf = Vec::new();
                        if signable {
                            let mut msg = v.clone();
                            msg.signature = None;
                            prost::Message::encode(&msg, &mut buf).expect("prost encode failed");
                        } else {
                            prost::Message::encode(v, &mut buf).expect("prost encode failed");
                        }
                        buf
                    }
                )+
                // Non-signable variants (encode directly, signable flag ignored)
                $(
                    ProtocolRecord::$plain_variant(v) => {
                        let mut buf = Vec::new();
                        prost::Message::encode(v, &mut buf).expect("prost encode failed");
                        buf
                    }
                )+
            }
        }};
    }

    encode_all! {
        signable: [
            EventEnvelope, CommandEnvelope, DelegationRecord, RevocationRecord,
            SnapshotDescriptor, IdentityRecord, AssuranceClaim, QueryResultFragment,
            QueryRequest, FederatedAggregateDescriptor,
            RouteAdvertisement, SessionHello, SessionAccept, RelayEnvelope,
        ],
        plain: [
            IdentityRef, NodeRef, StreamRef, EventRef, HeadRef, CheckpointRef,
            ObjectRef, RepresentationRef, CommandRef, DelegationRef, RevocationRef,
            SnapshotRef, Digest, Signature, TimeWindow, CostLimit, ScopeDescriptor,
            NodeGenesisPayload,
            CommandSentPayload, CommandResultPayload, ActionLifecyclePayload,
            SecretPutPayload, SecretDeletePayload,
            CollectionCreatedPayload, CollectionDeletedPayload,
            LogicalObjectDescriptor, StoredRepresentationHeader, ChunkEntry,
            ChunkManifest, AggregateTrustPolicy, RouteTrustAssignment,
            RouteTrustAssignments, RouteSelectionPolicy, AssuranceRequirement,
            ConstraintSet, CapabilityDescriptor, StreamHeadsProof, SnapshotSetProof,
            EventSetProof, ObjectAssertionProof, ResultFragmentProof,
            AggregateSummaryProof, TrustPolicyProof, ProofBundle, ReachabilityHint,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Signable types: signable=true clears signature, signable=false keeps it ----

    fn test_sig(marker: u8) -> Signature {
        Signature {
            algorithm: edgerun_proto::edgerun::v0::common::signature::Algorithm::SignatureAlgorithmEcdsaP256Sha256 as i32,
            value: vec![marker; 64],
        }
    }

    #[test]
    fn canonical_event_envelope_signable_clears_signature() {
        let event = EventEnvelope {
            envelope_version: 1,
            stream_id: b"stream-1".to_vec(),
            seq: 1,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: Some(test_sig(0xAB)),
        };

        let signable_bytes = canonical_bytes(&ProtocolRecord::EventEnvelope(event.clone()), true);
        let full_bytes = canonical_bytes(&ProtocolRecord::EventEnvelope(event.clone()), false);

        assert!(!signable_bytes.contains(&0xAB));
        assert!(full_bytes.contains(&0xAB));
        assert!(signable_bytes.len() < full_bytes.len());
    }

    #[test]
    fn canonical_event_envelope_signable_deterministic() {
        let event = EventEnvelope {
            envelope_version: 1,
            stream_id: b"stream-1".to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        let a = canonical_bytes(&ProtocolRecord::EventEnvelope(event.clone()), true);
        let b = canonical_bytes(&ProtocolRecord::EventEnvelope(event.clone()), true);
        assert_eq!(a, b);
    }

    #[test]
    fn canonical_command_envelope_signable_clears_signature() {
        let cmd = CommandEnvelope {
            envelope_version: 1,
            command_id: b"cmd-1".to_vec(),
            target_node: Some(NodeRef {
                node_id: b"node-1".to_vec(),
            }),
            issuer: Some(IdentityRef {
                identity_id: b"identity-1".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            command_type: CommandType::Query as i32,
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: vec![],
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: Some(test_sig(0xCD)),
        };

        let signable_bytes = canonical_bytes(&ProtocolRecord::CommandEnvelope(cmd.clone()), true);
        let full_bytes = canonical_bytes(&ProtocolRecord::CommandEnvelope(cmd.clone()), false);

        assert!(!signable_bytes.contains(&0xCD));
        assert!(full_bytes.contains(&0xCD));
        assert!(signable_bytes.len() < full_bytes.len());
    }

    #[test]
    fn canonical_delegation_record_signable() {
        let del = DelegationRecord {
            record_version: 1,
            delegation_id: b"del-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"iss".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: b"rec".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(test_sig(0xEE)),
        };

        let signable_bytes = canonical_bytes(&ProtocolRecord::DelegationRecord(del.clone()), true);
        let full_bytes = canonical_bytes(&ProtocolRecord::DelegationRecord(del.clone()), false);

        assert!(!signable_bytes.contains(&0xEE));
        assert!(full_bytes.contains(&0xEE));
    }

    #[test]
    fn canonical_revocation_record_signable() {
        let rev = RevocationRecord {
            record_version: 1,
            revocation_id: b"rev-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"iss".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: None,
            effective_at: None,
            revocation_kind: 0,
            target: None,
            scope_override: None,
            reason_code: String::new(),
            replacement_id: vec![],
            revocation_metadata: None,
            signature: Some(test_sig(0xFF)),
        };

        let signable_bytes = canonical_bytes(&ProtocolRecord::RevocationRecord(rev.clone()), true);
        let full_bytes = canonical_bytes(&ProtocolRecord::RevocationRecord(rev.clone()), false);

        assert!(!signable_bytes.contains(&0xFF));
        assert!(full_bytes.contains(&0xFF));
    }

    #[test]
    fn canonical_snapshot_descriptor_signable() {
        let snap = SnapshotDescriptor {
            descriptor_version: 1,
            snapshot_id: b"snap-1".to_vec(),
            view_type: String::new(),
            view_version: 1,
            producer: Some(IdentityRef {
                identity_id: b"prod".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            produced_at: None,
            base_heads: vec![],
            base_checkpoints: vec![],
            scope: None,
            completeness: 0,
            payload_object: None,
            supersedes: None,
            snapshot_metadata: None,
            signature: Some(test_sig(0xAA)),
        };

        let signable_bytes =
            canonical_bytes(&ProtocolRecord::SnapshotDescriptor(snap.clone()), true);
        let full_bytes = canonical_bytes(&ProtocolRecord::SnapshotDescriptor(snap.clone()), false);

        assert!(!signable_bytes.contains(&0xAA));
        assert!(full_bytes.contains(&0xAA));
    }

    #[test]
    fn canonical_query_result_fragment_signable_clears_signature() {
        let qrf = QueryResultFragment {
            fragment_version: 1,
            query_id: b"q-1".to_vec(),
            responder: None,
            answered_at: None,
            completeness: 0,
            snapshot_refs: vec![],
            event_refs: vec![],
            object_refs: vec![],
            proof_objects: vec![],
            omission_reason: String::new(),
            bundled_result_object: None,
            result_metadata: None,
            signature: Some(test_sig(0xBB)),
        };

        let signable_bytes =
            canonical_bytes(&ProtocolRecord::QueryResultFragment(qrf.clone()), true);
        let full_bytes = canonical_bytes(&ProtocolRecord::QueryResultFragment(qrf.clone()), false);

        assert!(!signable_bytes.contains(&0xBB));
        assert!(full_bytes.contains(&0xBB));
    }

    #[test]
    fn canonical_route_advertisement_signable_clears_signature() {
        let ra = RouteAdvertisement {
            advertisement_version: 1,
            target_node: None,
            advertiser: None,
            next_hop_node: None,
            reachability: vec![],
            metric_hint: None,
            advertised_at: None,
            expires_at: None,
            route_metadata: None,
            signature: Some(test_sig(0xCC)),
        };

        let signable_bytes = canonical_bytes(&ProtocolRecord::RouteAdvertisement(ra.clone()), true);
        let full_bytes = canonical_bytes(&ProtocolRecord::RouteAdvertisement(ra.clone()), false);

        assert!(!signable_bytes.contains(&0xCC));
        assert!(full_bytes.contains(&0xCC));
    }

    #[test]
    fn canonical_session_hello_signable_clears_signature() {
        let sh = SessionHello {
            message_version: 1,
            initiator: None,
            target_node: None,
            supported_transport_features: vec![],
            supported_protocol_versions: vec![],
            session_nonce: vec![0x01; 32],
            initiator_locators: vec![],
            hello_metadata: None,
            signature: Some(test_sig(0xDD)),
        };

        let signable_bytes = canonical_bytes(&ProtocolRecord::SessionHello(sh.clone()), true);
        let full_bytes = canonical_bytes(&ProtocolRecord::SessionHello(sh.clone()), false);

        assert!(!signable_bytes.contains(&0xDD));
        assert!(full_bytes.contains(&0xDD));
    }

    #[test]
    fn canonical_session_accept_signable_clears_signature() {
        let sa = SessionAccept {
            message_version: 1,
            responder: None,
            echoed_session_nonce: vec![0x01; 32],
            selected_protocol_version: 1,
            selected_transport_features: vec![],
            responder_locators: vec![],
            accept_metadata: None,
            signature: Some(test_sig(0xDE)),
        };

        let signable_bytes = canonical_bytes(&ProtocolRecord::SessionAccept(sa.clone()), true);
        let full_bytes = canonical_bytes(&ProtocolRecord::SessionAccept(sa.clone()), false);

        assert!(!signable_bytes.contains(&0xDE));
        assert!(full_bytes.contains(&0xDE));
    }

    #[test]
    fn canonical_relay_envelope_signable_clears_signature() {
        let re_ = RelayEnvelope {
            envelope_version: 1,
            relay_message_id: b"relay-1".to_vec(),
            original_sender: Some(IdentityRef {
                identity_id: b"sender".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            intended_recipient_node: Some(NodeRef {
                node_id: b"target".to_vec(),
            }),
            relay_chain: vec![],
            payload_kind: 0,
            payload: None,
            store_until: None,
            relay_metadata: None,
            signature: Some(test_sig(0xDF)),
        };

        let signable_bytes = canonical_bytes(&ProtocolRecord::RelayEnvelope(re_.clone()), true);
        let full_bytes = canonical_bytes(&ProtocolRecord::RelayEnvelope(re_.clone()), false);

        assert!(!signable_bytes.contains(&0xDF));
        assert!(full_bytes.contains(&0xDF));
    }

    #[test]
    fn canonical_identity_record_signable_clears_signature() {
        let ir = IdentityRecord {
            record_version: 1,
            identity_id: b"identity-1".to_vec(),
            identity_kind: 0,
            key_algorithm: 0,
            public_key: vec![0x04; 65],
            created_at: None,
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: Some(test_sig(0xE0)),
        };

        let signable_bytes = canonical_bytes(&ProtocolRecord::IdentityRecord(ir.clone()), true);
        let full_bytes = canonical_bytes(&ProtocolRecord::IdentityRecord(ir.clone()), false);

        assert!(!signable_bytes.contains(&0xE0));
        assert!(full_bytes.contains(&0xE0));
    }

    #[test]
    fn canonical_query_request_signable_clears_signature() {
        let qr = QueryRequest {
            request_version: 1,
            query_id: b"q-1".to_vec(),
            requester: Some(IdentityRef {
                identity_id: b"req".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            target_scope: None,
            query_class: 0,
            time_window: None,
            checkpoint_base: None,
            result_limit: None,
            cost_limit: None,
            required_proof_classes: vec![],
            query_payload_object: None,
            signature: Some(test_sig(0xE1)),
        };

        let signable_bytes = canonical_bytes(&ProtocolRecord::QueryRequest(qr.clone()), true);
        let full_bytes = canonical_bytes(&ProtocolRecord::QueryRequest(qr.clone()), false);

        assert!(!signable_bytes.contains(&0xE1));
        assert!(full_bytes.contains(&0xE1));
    }

    #[test]
    fn canonical_federated_aggregate_descriptor_signable_clears_signature() {
        let fad = FederatedAggregateDescriptor {
            descriptor_version: 1,
            aggregate_id: b"agg-1".to_vec(),
            source_query_id: b"q-1".to_vec(),
            aggregator: Some(IdentityRef {
                identity_id: b"agg".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            aggregated_at: None,
            input_fragments: vec![],
            aggregation_policy_object: None,
            payload_object: None,
            signature: Some(test_sig(0xE2)),
        };

        let signable_bytes = canonical_bytes(
            &ProtocolRecord::FederatedAggregateDescriptor(fad.clone()),
            true,
        );
        let full_bytes = canonical_bytes(
            &ProtocolRecord::FederatedAggregateDescriptor(fad.clone()),
            false,
        );

        assert!(!signable_bytes.contains(&0xE2));
        assert!(full_bytes.contains(&0xE2));
    }

    // ---- Non-signable types: signable flag has no effect ----

    #[test]
    fn canonical_non_signable_types_ignore_signable_flag() {
        // Test a representative sample of non-signable types
        let node_ref = ProtocolRecord::NodeRef(NodeRef {
            node_id: b"n".to_vec(),
        });
        assert_eq!(
            canonical_bytes(&node_ref, true),
            canonical_bytes(&node_ref, false)
        );

        let identity_ref = ProtocolRecord::IdentityRef(IdentityRef {
            identity_id: b"i".to_vec(),
            identity_kind: Some(0),
            key_hint: None,
        });
        assert_eq!(
            canonical_bytes(&identity_ref, true),
            canonical_bytes(&identity_ref, false)
        );

        let object_ref = ProtocolRecord::ObjectRef(ObjectRef {
            object_id: b"o".to_vec(),
            object_kind: None,
        });
        assert_eq!(
            canonical_bytes(&object_ref, true),
            canonical_bytes(&object_ref, false)
        );

        let event_ref = ProtocolRecord::EventRef(EventRef {
            stream_id: b"s".to_vec(),
            seq: 1,
            event_hash: None,
        });
        assert_eq!(
            canonical_bytes(&event_ref, true),
            canonical_bytes(&event_ref, false)
        );

        let head_ref = ProtocolRecord::HeadRef(HeadRef {
            stream_id: b"s".to_vec(),
            seq: 1,
            event_hash: None,
        });
        assert_eq!(
            canonical_bytes(&head_ref, true),
            canonical_bytes(&head_ref, false)
        );

        let checkpoint_ref = ProtocolRecord::CheckpointRef(CheckpointRef {
            checkpoint_id: None,
            heads: vec![],
        });
        assert_eq!(
            canonical_bytes(&checkpoint_ref, true),
            canonical_bytes(&checkpoint_ref, false)
        );

        let command_ref = ProtocolRecord::CommandRef(CommandRef {
            command_id: b"c".to_vec(),
            command_hash: None,
        });
        assert_eq!(
            canonical_bytes(&command_ref, true),
            canonical_bytes(&command_ref, false)
        );

        let delegation_ref = ProtocolRecord::DelegationRef(DelegationRef {
            delegation_id: b"d".to_vec(),
            delegation_hash: None,
        });
        assert_eq!(
            canonical_bytes(&delegation_ref, true),
            canonical_bytes(&delegation_ref, false)
        );

        let revocation_ref = ProtocolRecord::RevocationRef(RevocationRef {
            revocation_id: b"r".to_vec(),
            revocation_hash: None,
        });
        assert_eq!(
            canonical_bytes(&revocation_ref, true),
            canonical_bytes(&revocation_ref, false)
        );

        let snapshot_ref = ProtocolRecord::SnapshotRef(SnapshotRef {
            snapshot_id: b"s".to_vec(),
            object_id: None,
        });
        assert_eq!(
            canonical_bytes(&snapshot_ref, true),
            canonical_bytes(&snapshot_ref, false)
        );

        let stream_ref = ProtocolRecord::StreamRef(StreamRef {
            stream_id: b"s".to_vec(),
        });
        assert_eq!(
            canonical_bytes(&stream_ref, true),
            canonical_bytes(&stream_ref, false)
        );

        let representation_ref = ProtocolRecord::RepresentationRef(RepresentationRef {
            representation_id: b"r".to_vec(),
            object_id: b"o".to_vec(),
        });
        assert_eq!(
            canonical_bytes(&representation_ref, true),
            canonical_bytes(&representation_ref, false)
        );

        let time_window = ProtocolRecord::TimeWindow(TimeWindow {
            not_before: None,
            expires_at: None,
        });
        assert_eq!(
            canonical_bytes(&time_window, true),
            canonical_bytes(&time_window, false)
        );

        let cost_limit = ProtocolRecord::CostLimit(CostLimit {
            max_results: None,
            max_total_bytes: None,
            max_wall_time: None,
            max_federated_responders: None,
        });
        assert_eq!(
            canonical_bytes(&cost_limit, true),
            canonical_bytes(&cost_limit, false)
        );

        let scope = ProtocolRecord::ScopeDescriptor(ScopeDescriptor {
            scope_version: 1,
            scope_kind: 0,
            target_nodes: vec![],
            target_streams: vec![],
            target_object_kinds: vec![],
            target_view_types: vec![],
            target_domains: vec![],
            time_bounds: None,
            scope_metadata: None,
        });
        assert_eq!(
            canonical_bytes(&scope, true),
            canonical_bytes(&scope, false)
        );
    }

    #[test]
    fn canonical_encoding_is_non_empty_and_deterministic() {
        // Test that encoding works for all ProtocolRecord variants and produces deterministic output
        let ngp = ProtocolRecord::NodeGenesisPayload(NodeGenesisPayload {
            payload_version: 1,
            node_id: b"n".to_vec(),
            primary_node_identity: Some(IdentityRef {
                identity_id: b"i".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            initial_controllers: vec![],
            initial_policy_object: None,
            bootstrap_records: vec![],
            assurance_claims: vec![],
            node_roles: vec![],
            genesis_metadata: None,
        });
        let a = canonical_bytes(&ngp, true);
        let b = canonical_bytes(&ngp, true);
        assert!(!a.is_empty());
        assert_eq!(a, b);

        let csp = ProtocolRecord::CommandSentPayload(CommandSentPayload {
            payload_version: 1,
            command: Some(CommandRef {
                command_id: b"c".to_vec(),
                command_hash: None,
            }),
            target_node: Some(NodeRef {
                node_id: b"n".to_vec(),
            }),
            send_metadata: None,
        });
        assert!(!canonical_bytes(&csp, true).is_empty());

        let crp = ProtocolRecord::CommandResultPayload(CommandResultPayload {
            payload_version: 1,
            command: Some(CommandRef {
                command_id: b"c".to_vec(),
                command_hash: None,
            }),
            issuer: Some(IdentityRef {
                identity_id: b"i".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            decision: 0,
            decision_basis: None,
            reason_code: String::new(),
            effect_summary_object: None,
            result_object: None,
        });
        assert!(!canonical_bytes(&crp, true).is_empty());

        let alp = ProtocolRecord::ActionLifecyclePayload(ActionLifecyclePayload {
            payload_version: 1,
            origin_command: Some(CommandRef {
                command_id: b"c".to_vec(),
                command_hash: None,
            }),
            action_instance_id: b"a".to_vec(),
            status: 0,
            result_object: None,
            error_object: None,
            progress_object: None,
            action_metadata: None,
        });
        assert!(!canonical_bytes(&alp, true).is_empty());

        let lod = ProtocolRecord::LogicalObjectDescriptor(LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: b"o".to_vec(),
            object_kind: 0,
            object_schema_version: 1,
            canonicalization_id: String::new(),
            canonical_digest: None,
            canonical_size: 0,
            created_at: None,
            producer: None,
            describes_object: None,
            object_metadata: None,
        });
        assert!(!canonical_bytes(&lod, true).is_empty());

        let srh = ProtocolRecord::StoredRepresentationHeader(StoredRepresentationHeader {
            header_version: 1,
            representation_id: b"r".to_vec(),
            object: Some(ObjectRef {
                object_id: b"o".to_vec(),
                object_kind: None,
            }),
            representation_digest: None,
            plaintext_size: None,
            stored_size: 0,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: 0,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        });
        assert!(!canonical_bytes(&srh, true).is_empty());

        let ce = ProtocolRecord::ChunkEntry(ChunkEntry {
            index: 0,
            chunk_representation_id: b"c".to_vec(),
            chunk_digest: None,
            offset: 0,
            length: 0,
        });
        assert!(!canonical_bytes(&ce, true).is_empty());

        let cm = ProtocolRecord::ChunkManifest(ChunkManifest {
            manifest_version: 1,
            object: Some(ObjectRef {
                object_id: b"o".to_vec(),
                object_kind: None,
            }),
            representation: None,
            chunk_count: 0,
            total_stored_size: 0,
            chunk_entries: vec![],
            manifest_metadata: None,
        });
        assert!(!canonical_bytes(&cm, true).is_empty());

        let atp = ProtocolRecord::AggregateTrustPolicy(AggregateTrustPolicy {
            policy_version: 1,
            minimum_trust_score: None,
            preferred_aggregators: vec![],
            allowed_responders: vec![],
            policy_metadata: None,
        });
        assert!(!canonical_bytes(&atp, true).is_empty());

        let rta = ProtocolRecord::RouteTrustAssignment(RouteTrustAssignment {
            subject: None,
            trust_score: 0,
            source: String::new(),
        });
        // RouteTrustAssignment with all default/empty fields encodes to zero bytes
        // (all fields are default values that get omitted in protobuf)
        let _ = canonical_bytes(&rta, true);

        let rts = ProtocolRecord::RouteTrustAssignments(RouteTrustAssignments {
            assignments: vec![],
        });
        let _ = canonical_bytes(&rts, true);

        let rsp = ProtocolRecord::RouteSelectionPolicy(RouteSelectionPolicy {
            policy_version: 1,
            minimum_quality_hint: None,
            maximum_cost_hint: None,
            preferred_advertisers: vec![],
            preferred_next_hops: vec![],
            require_active_session: None,
            policy_metadata: None,
        });
        assert!(!canonical_bytes(&rsp, true).is_empty());

        let ar = ProtocolRecord::AssuranceRequirement(AssuranceRequirement {
            assurance_version: 1,
            required_class: 0,
            acceptable_attesters: vec![],
            max_evidence_age: None,
            assurance_metadata: None,
        });
        assert!(!canonical_bytes(&ar, true).is_empty());

        let cs = ProtocolRecord::ConstraintSet(ConstraintSet {
            constraint_version: 1,
            not_before: None,
            expires_at: None,
            max_uses: None,
            rate_limit: None,
            requires_local_session: None,
            requires_user_presence: None,
            requires_transport_classes: vec![],
            requires_location_classes: vec![],
            export_policy: 0,
            execution_class_limits: vec![],
            storage_class_limits: vec![],
            constraint_metadata: None,
        });
        assert!(!canonical_bytes(&cs, true).is_empty());

        let cd = ProtocolRecord::CapabilityDescriptor(CapabilityDescriptor {
            capability_version: 1,
            capability_kind: 0,
            actions: vec![],
            scope: None,
            constraints: None,
            delegation_policy: 0,
            minimum_assurance: None,
            capability_metadata: None,
        });
        assert!(!canonical_bytes(&cd, true).is_empty());

        let shp = ProtocolRecord::StreamHeadsProof(StreamHeadsProof {
            source_query_id: b"q".to_vec(),
            heads: vec![],
        });
        assert!(!canonical_bytes(&shp, true).is_empty());

        let ssp = ProtocolRecord::SnapshotSetProof(SnapshotSetProof {
            source_query_id: b"q".to_vec(),
            snapshots: vec![],
        });
        assert!(!canonical_bytes(&ssp, true).is_empty());

        let esp = ProtocolRecord::EventSetProof(EventSetProof {
            source_query_id: b"q".to_vec(),
            events: vec![],
            related_objects: vec![],
        });
        assert!(!canonical_bytes(&esp, true).is_empty());

        let oap = ProtocolRecord::ObjectAssertionProof(ObjectAssertionProof {
            source_query_id: b"q".to_vec(),
            object_ref: None,
            exists: false,
            bundled_result_object: None,
        });
        assert!(!canonical_bytes(&oap, true).is_empty());

        let rfp = ProtocolRecord::ResultFragmentProof(ResultFragmentProof { fragment: None });
        let _ = canonical_bytes(&rfp, true);

        let asp = ProtocolRecord::AggregateSummaryProof(AggregateSummaryProof {
            source_query_id: b"q".to_vec(),
            included_responders: vec![],
            excluded_responders: vec![],
            total_trust_score: 0,
            trust_policy_object: None,
        });
        assert!(!canonical_bytes(&asp, true).is_empty());

        let tpp = ProtocolRecord::TrustPolicyProof(TrustPolicyProof {
            source_query_id: b"q".to_vec(),
            policy_object: None,
            assignments_object: None,
        });
        assert!(!canonical_bytes(&tpp, true).is_empty());

        let pb = ProtocolRecord::ProofBundle(ProofBundle {
            bundle_version: 1,
            payload_type: 0,
            source_query_id: b"q".to_vec(),
            payload_object: None,
            supporting_objects: vec![],
            signature: None,
        });
        assert!(!canonical_bytes(&pb, true).is_empty());

        let fad = ProtocolRecord::FederatedAggregateDescriptor(FederatedAggregateDescriptor {
            descriptor_version: 1,
            aggregate_id: b"a".to_vec(),
            source_query_id: b"q".to_vec(),
            aggregator: None,
            aggregated_at: None,
            input_fragments: vec![],
            aggregation_policy_object: None,
            payload_object: None,
            signature: None,
        });
        assert!(!canonical_bytes(&fad, true).is_empty());

        let rh = ProtocolRecord::ReachabilityHint(ReachabilityHint {
            hint_version: 1,
            subject_node: None,
            transport_class: 0,
            locator_payload: vec![0x01],
            directness: 0,
            valid_after: None,
            valid_until: None,
            cost_hint: None,
            quality_hint: None,
            issuer: None,
            signature: None,
        });
        assert!(!canonical_bytes(&rh, true).is_empty());

        let ac = ProtocolRecord::AssuranceClaim(AssuranceClaim {
            claim_version: 1,
            subject: None,
            assurance_class: 0,
            attester: None,
            issued_at: None,
            expires_at: None,
            evidence_object: None,
            claim_note: String::new(),
            signature: None,
        });
        // AssuranceClaim is signable but has no signature, so both forms should be identical
        let a_bytes = canonical_bytes(&ac, true);
        let b_bytes = canonical_bytes(&ac, false);
        assert!(!a_bytes.is_empty());
        assert_eq!(a_bytes, b_bytes);

        // Digest and Signature types
        let d = ProtocolRecord::Digest(Digest {
            algorithm: edgerun_proto::edgerun::v0::common::digest::Algorithm::DigestAlgorithmSha256
                as i32,
            value: vec![0x01; 32],
        });
        assert!(!canonical_bytes(&d, true).is_empty());

        let s = ProtocolRecord::Signature(test_sig(0x02));
        assert!(!canonical_bytes(&s, true).is_empty());
    }

    #[test]
    fn canonical_bytes_with_none_signature_is_identical_both_ways() {
        // When signature is None, signable=true and signable=false should produce identical bytes
        let event = EventEnvelope {
            envelope_version: 1,
            stream_id: b"stream-1".to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        let a = canonical_bytes(&ProtocolRecord::EventEnvelope(event.clone()), true);
        let b = canonical_bytes(&ProtocolRecord::EventEnvelope(event.clone()), false);
        assert_eq!(a, b);
    }
}
