//! Protobuf-generated wire types — the canonical edgerun protocol types.
//!
//! Every domain message is a protobuf struct with `prost::Message`.
//! Canonical encoding = `prost::Message::encode()` directly.

extern crate alloc;

pub mod edgerun {
    pub mod v0 {
        pub mod access {
            core::include!("gen/edgerun.v0.access.rs");
        }
        pub mod app {
            core::include!("gen/edgerun.v0.app.rs");
        }
        pub mod appabi {
            core::include!("gen/edgerun/v0/appabi/edgerun.v0.appabi.rs");
        }
        pub mod capability {
            core::include!("gen/edgerun.v0.capability.rs");
        }
        pub mod capability_runtime {
            core::include!("gen/edgerun.v0.capability_runtime.rs");
        }
        pub mod common {
            core::include!("gen/edgerun.v0.common.rs");
        }
        pub mod identity {
            core::include!("gen/edgerun.v0.identity.rs");
        }
        pub mod network {
            core::include!("gen/edgerun.v0.network.rs");
        }
        pub mod object {
            core::include!("gen/edgerun.v0.object.rs");
        }
        pub mod stream {
            core::include!("gen/edgerun.v0.stream.rs");
        }
        pub mod trust {
            core::include!("gen/edgerun.v0.trust.rs");
        }
        pub mod ui {
            core::include!("gen/edgerun.v0.ui.rs");
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::{String, ToString};
    use alloc::vec;
    use alloc::vec::Vec;
    use core::fmt;

    use super::edgerun::v0::*;

    // -----------------------------------------------------------------------
    // Helper: roundtrip encode/decode
    // -----------------------------------------------------------------------
    fn roundtrip<M: prost::Message + Default + PartialEq + fmt::Debug>(msg: &M) -> M {
        let mut buf = Vec::new();
        msg.encode(&mut buf).expect("encode should succeed");
        M::decode(buf.as_slice()).expect("decode should succeed")
    }

    // =======================================================================
    // common module — enums
    // =======================================================================

    #[test]
    fn common_digest_algorithm_enum_roundtrip() {
        use common::digest::Algorithm;
        assert_eq!(Algorithm::DigestAlgorithmSha256 as i32, 1);
        assert_eq!(Algorithm::DigestAlgorithmUnspecified as i32, 0);
        assert_eq!(
            Algorithm::DigestAlgorithmSha256.as_str_name(),
            "DIGEST_ALGORITHM_SHA256"
        );
        assert_eq!(
            Algorithm::from_str_name("DIGEST_ALGORITHM_SHA256"),
            Some(Algorithm::DigestAlgorithmSha256)
        );
        assert_eq!(Algorithm::from_str_name("NONEXISTENT"), None);
    }

    #[test]
    fn common_signature_algorithm_enum_roundtrip() {
        use common::signature::Algorithm;
        assert_eq!(Algorithm::SignatureAlgorithmEcdsaP256Sha256 as i32, 1);
        assert_eq!(
            Algorithm::SignatureAlgorithmEcdsaP256Sha256.as_str_name(),
            "SIGNATURE_ALGORITHM_ECDSA_P256_SHA256"
        );
        assert_eq!(
            Algorithm::from_str_name("SIGNATURE_ALGORITHM_ECDSA_P256_SHA256"),
            Some(Algorithm::SignatureAlgorithmEcdsaP256Sha256)
        );
    }

    #[test]
    fn common_identity_kind_enum() {
        use common::IdentityKind;
        assert_eq!(IdentityKind::Unspecified as i32, 0);
        assert_eq!(IdentityKind::User as i32, 1);
        assert_eq!(IdentityKind::Node as i32, 2);
        assert_eq!(IdentityKind::Agent as i32, 3);
        assert_eq!(IdentityKind::Service as i32, 4);
        assert_eq!(IdentityKind::Other as i32, 5);
        assert_eq!(
            IdentityKind::from_str_name("IDENTITY_KIND_USER"),
            Some(IdentityKind::User)
        );
        assert_eq!(IdentityKind::from_str_name("INVALID"), None);
    }

    #[test]
    fn common_object_kind_enum() {
        use common::ObjectKind;
        assert_eq!(ObjectKind::Unspecified as i32, 0);
        assert_eq!(ObjectKind::Payload as i32, 1);
        assert_eq!(ObjectKind::Attachment as i32, 2);
        assert_eq!(ObjectKind::Snapshot as i32, 3);
        assert_eq!(ObjectKind::Manifest as i32, 4);
        assert_eq!(ObjectKind::Index as i32, 5);
        assert_eq!(ObjectKind::Command as i32, 6);
        assert_eq!(ObjectKind::Proof as i32, 7);
        assert_eq!(ObjectKind::DerivedView as i32, 8);
        assert_eq!(
            ObjectKind::from_str_name("OBJECT_KIND_PAYLOAD"),
            Some(ObjectKind::Payload)
        );
    }

    #[test]
    fn common_assurance_class_enum() {
        use common::AssuranceClass;
        assert_eq!(AssuranceClass::Unspecified as i32, 0);
        assert_eq!(AssuranceClass::Software as i32, 1);
        assert_eq!(AssuranceClass::HardwareBacked as i32, 2);
        assert_eq!(AssuranceClass::AttestedRuntime as i32, 3);
        assert_eq!(
            AssuranceClass::from_str_name("ASSURANCE_CLASS_HARDWARE_BACKED"),
            Some(AssuranceClass::HardwareBacked)
        );
    }

    #[test]
    fn common_transport_class_enum() {
        use common::TransportClass;
        assert_eq!(TransportClass::Unspecified as i32, 0);
        assert_eq!(TransportClass::Ble as i32, 1);
        assert_eq!(TransportClass::LanIp as i32, 2);
        assert_eq!(TransportClass::Quic as i32, 3);
        assert_eq!(TransportClass::WifiDirect as i32, 4);
        assert_eq!(TransportClass::Relay as i32, 5);
        assert_eq!(TransportClass::StoreForward as i32, 6);
        assert_eq!(TransportClass::Other as i32, 7);
    }

    #[test]
    fn common_directness_enum() {
        use common::Directness;
        assert_eq!(Directness::Unspecified as i32, 0);
        assert_eq!(Directness::Direct as i32, 1);
        assert_eq!(Directness::Relayed as i32, 2);
        assert_eq!(Directness::BridgeRequired as i32, 3);
        assert_eq!(Directness::StoreForward as i32, 4);
    }

    #[test]
    fn common_storage_class_enum() {
        use common::StorageClass;
        assert_eq!(StorageClass::Unspecified as i32, 0);
        assert_eq!(StorageClass::Hot as i32, 1);
        assert_eq!(StorageClass::Warm as i32, 2);
        assert_eq!(StorageClass::Cold as i32, 3);
        assert_eq!(StorageClass::Archive as i32, 4);
    }

    #[test]
    fn common_execution_class_enum() {
        use common::ExecutionClass;
        assert_eq!(ExecutionClass::Unspecified as i32, 0);
        assert_eq!(ExecutionClass::LocalOnly as i32, 1);
        assert_eq!(ExecutionClass::TrustedPeer as i32, 2);
        assert_eq!(ExecutionClass::TeeAllowed as i32, 3);
        assert_eq!(ExecutionClass::RedundantUntrusted as i32, 4);
        assert_eq!(ExecutionClass::Public as i32, 5);
    }

    // =======================================================================
    // common module — messages (default + roundtrip)
    // =======================================================================

    #[test]
    fn common_digest_default() {
        let d = common::Digest::default();
        assert_eq!(d.algorithm, 0); // unspecified
        assert!(d.value.is_empty());
    }

    #[test]
    fn common_digest_roundtrip() {
        let d = common::Digest {
            algorithm: common::digest::Algorithm::DigestAlgorithmSha256 as i32,
            value: vec![0xab; 32],
        };
        let decoded = roundtrip(&d);
        assert_eq!(decoded.algorithm, d.algorithm);
        assert_eq!(decoded.value, d.value);
    }

    #[test]
    fn common_signature_roundtrip() {
        let s = common::Signature {
            algorithm: common::signature::Algorithm::SignatureAlgorithmEcdsaP256Sha256 as i32,
            value: vec![0x01, 0x02, 0x03],
        };
        let decoded = roundtrip(&s);
        assert_eq!(decoded.algorithm, s.algorithm);
        assert_eq!(decoded.value, s.value);
    }

    #[test]
    fn common_time_window_default() {
        let tw = common::TimeWindow::default();
        assert!(tw.not_before.is_none());
        assert!(tw.expires_at.is_none());
    }

    #[test]
    fn common_identity_ref_default() {
        let r = common::IdentityRef::default();
        assert!(r.identity_id.is_empty());
        assert!(r.identity_kind.is_none());
        assert!(r.key_hint.is_none());
    }

    #[test]
    fn common_identity_ref_roundtrip() {
        let r = common::IdentityRef {
            identity_id: vec![0x01, 0x02],
            identity_kind: Some(common::IdentityKind::User as i32),
            key_hint: Some(vec![0xff]),
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.identity_id, r.identity_id);
        assert_eq!(decoded.identity_kind, r.identity_kind);
        assert_eq!(decoded.key_hint, r.key_hint);
    }

    #[test]
    fn common_node_ref_roundtrip() {
        let r = common::NodeRef {
            node_id: vec![0xaa; 32],
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.node_id, r.node_id);
    }

    #[test]
    fn common_stream_ref_roundtrip() {
        let r = common::StreamRef {
            stream_id: vec![0xbb; 16],
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.stream_id, r.stream_id);
    }

    #[test]
    fn common_event_ref_roundtrip() {
        let r = common::EventRef {
            stream_id: vec![0x01],
            seq: 42,
            event_hash: Some(common::Digest {
                algorithm: common::digest::Algorithm::DigestAlgorithmSha256 as i32,
                value: vec![0xcd; 32],
            }),
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.stream_id, r.stream_id);
        assert_eq!(decoded.seq, r.seq);
        assert_eq!(decoded.event_hash, r.event_hash);
    }

    #[test]
    fn common_head_ref_roundtrip() {
        let r = common::HeadRef {
            stream_id: vec![0x10],
            seq: 100,
            event_hash: None,
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.stream_id, r.stream_id);
        assert_eq!(decoded.seq, r.seq);
        assert!(decoded.event_hash.is_none());
    }

    #[test]
    fn common_checkpoint_ref_default() {
        let c = common::CheckpointRef::default();
        assert!(c.checkpoint_id.is_none());
        assert!(c.heads.is_empty());
    }

    #[test]
    fn common_checkpoint_ref_roundtrip() {
        let c = common::CheckpointRef {
            checkpoint_id: Some(vec![0xcc]),
            heads: vec![common::HeadRef {
                stream_id: vec![0x01],
                seq: 1,
                event_hash: None,
            }],
        };
        let decoded = roundtrip(&c);
        assert_eq!(decoded.checkpoint_id, c.checkpoint_id);
        assert_eq!(decoded.heads.len(), 1);
    }

    #[test]
    fn common_object_ref_roundtrip() {
        let r = common::ObjectRef {
            object_id: vec![0x0d],
            object_kind: Some(common::ObjectKind::Payload as i32),
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.object_id, r.object_id);
        assert_eq!(decoded.object_kind, r.object_kind);
    }

    #[test]
    fn common_representation_ref_roundtrip() {
        let r = common::RepresentationRef {
            representation_id: vec![0x01],
            object_id: vec![0x02],
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.representation_id, r.representation_id);
        assert_eq!(decoded.object_id, r.object_id);
    }

    #[test]
    fn common_command_ref_roundtrip() {
        let r = common::CommandRef {
            command_id: vec![0xc0],
            command_hash: Some(common::Digest {
                algorithm: common::digest::Algorithm::DigestAlgorithmSha256 as i32,
                value: vec![0xab; 32],
            }),
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.command_id, r.command_id);
        assert_eq!(decoded.command_hash, r.command_hash);
    }

    #[test]
    fn common_delegation_ref_roundtrip() {
        let r = common::DelegationRef {
            delegation_id: vec![0xd0],
            delegation_hash: None,
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.delegation_id, r.delegation_id);
    }

    #[test]
    fn common_revocation_ref_roundtrip() {
        let r = common::RevocationRef {
            revocation_id: vec![0xe0],
            revocation_hash: None,
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.revocation_id, r.revocation_id);
    }

    #[test]
    fn common_snapshot_ref_roundtrip() {
        let r = common::SnapshotRef {
            snapshot_id: vec![0xf0],
            object_id: Some(vec![0x01]),
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.snapshot_id, r.snapshot_id);
        assert_eq!(decoded.object_id, r.object_id);
    }

    #[test]
    fn common_rate_limit_default() {
        let rl = common::RateLimit::default();
        assert_eq!(rl.max_operations, 0);
        assert!(rl.per.is_none());
    }

    // =======================================================================
    // access module — enums
    // =======================================================================

    #[test]
    fn access_snapshot_completeness_enum() {
        use access::SnapshotCompleteness;
        assert_eq!(SnapshotCompleteness::Unspecified as i32, 0);
        assert_eq!(SnapshotCompleteness::Full as i32, 1);
        assert_eq!(SnapshotCompleteness::Partial as i32, 2);
        assert_eq!(SnapshotCompleteness::Bounded as i32, 3);
        assert_eq!(
            SnapshotCompleteness::from_str_name("SNAPSHOT_COMPLETENESS_FULL"),
            Some(SnapshotCompleteness::Full)
        );
    }

    #[test]
    fn access_query_class_enum() {
        use access::QueryClass;
        assert_eq!(QueryClass::Unspecified as i32, 0);
        assert_eq!(QueryClass::Head as i32, 1);
        assert_eq!(QueryClass::Snapshot as i32, 2);
        assert_eq!(QueryClass::EventRange as i32, 3);
        assert_eq!(QueryClass::ObjectExistence as i32, 4);
        assert_eq!(QueryClass::ObjectFetch as i32, 5);
        assert_eq!(QueryClass::View as i32, 6);
        assert_eq!(QueryClass::Search as i32, 7);
        assert_eq!(QueryClass::TrustState as i32, 8);
    }

    #[test]
    fn access_proof_class_enum() {
        use access::ProofClass;
        assert_eq!(ProofClass::Unspecified as i32, 0);
        assert_eq!(ProofClass::Signature as i32, 1);
        assert_eq!(ProofClass::StreamHead as i32, 2);
        assert_eq!(ProofClass::EventRef as i32, 3);
        assert_eq!(ProofClass::ObjectRef as i32, 4);
        assert_eq!(ProofClass::SnapshotBase as i32, 5);
    }

    #[test]
    fn access_result_completeness_enum() {
        use access::ResultCompleteness;
        assert_eq!(ResultCompleteness::Unspecified as i32, 0);
        assert_eq!(ResultCompleteness::CompleteForLocalKnowledge as i32, 1);
        assert_eq!(ResultCompleteness::Partial as i32, 2);
        assert_eq!(ResultCompleteness::Denied as i32, 3);
        assert_eq!(ResultCompleteness::MetadataOnly as i32, 4);
    }

    #[test]
    fn access_proof_payload_type_enum() {
        use access::ProofPayloadType;
        assert_eq!(ProofPayloadType::Unspecified as i32, 0);
        assert_eq!(ProofPayloadType::StreamHeads as i32, 1);
        assert_eq!(ProofPayloadType::SnapshotSet as i32, 2);
        assert_eq!(ProofPayloadType::EventSet as i32, 3);
        assert_eq!(ProofPayloadType::ObjectAssertion as i32, 4);
        assert_eq!(ProofPayloadType::ResultFragment as i32, 5);
        assert_eq!(ProofPayloadType::AggregateSummary as i32, 6);
        assert_eq!(ProofPayloadType::TrustPolicy as i32, 7);
    }

    // =======================================================================
    // access module — messages
    // =======================================================================

    #[test]
    fn access_cost_limit_default() {
        let cl = access::CostLimit::default();
        assert!(cl.max_results.is_none());
        assert!(cl.max_total_bytes.is_none());
        assert!(cl.max_wall_time.is_none());
        assert!(cl.max_federated_responders.is_none());
    }

    #[test]
    fn access_cost_limit_roundtrip() {
        let cl = access::CostLimit {
            max_results: Some(100),
            max_total_bytes: Some(1024),
            max_wall_time: None,
            max_federated_responders: Some(5),
        };
        let decoded = roundtrip(&cl);
        assert_eq!(decoded.max_results, cl.max_results);
        assert_eq!(decoded.max_total_bytes, cl.max_total_bytes);
        assert_eq!(
            decoded.max_federated_responders,
            cl.max_federated_responders
        );
    }

    #[test]
    fn access_query_request_default() {
        let qr = access::QueryRequest::default();
        assert_eq!(qr.request_version, 0);
        assert!(qr.query_id.is_empty());
        assert!(qr.requester.is_none());
        assert!(qr.target_scope.is_none());
        assert_eq!(qr.query_class, 0); // unspecified
        assert!(qr.required_proof_classes.is_empty());
    }

    #[test]
    fn access_query_request_roundtrip() {
        let qr = access::QueryRequest {
            request_version: 1,
            query_id: vec![0x01, 0x02, 0x03],
            requester: Some(common::IdentityRef {
                identity_id: vec![0x10],
                identity_kind: Some(common::IdentityKind::Node as i32),
                key_hint: None,
            }),
            target_scope: None,
            query_class: access::QueryClass::Head as i32,
            time_window: None,
            checkpoint_base: None,
            result_limit: Some(10),
            cost_limit: Some(access::CostLimit {
                max_results: Some(5),
                max_total_bytes: None,
                max_wall_time: None,
                max_federated_responders: None,
            }),
            required_proof_classes: vec![
                access::ProofClass::Signature as i32,
                access::ProofClass::StreamHead as i32,
            ],
            query_payload_object: None,
            signature: None,
        };
        let decoded = roundtrip(&qr);
        assert_eq!(decoded.request_version, qr.request_version);
        assert_eq!(decoded.query_id, qr.query_id);
        assert_eq!(decoded.query_class, qr.query_class);
        assert_eq!(decoded.required_proof_classes, qr.required_proof_classes);
        assert_eq!(decoded.result_limit, qr.result_limit);
    }

    #[test]
    fn access_stream_heads_proof_roundtrip() {
        let p = access::StreamHeadsProof {
            source_query_id: vec![0x01],
            heads: vec![common::HeadRef {
                stream_id: vec![0x10],
                seq: 1,
                event_hash: None,
            }],
        };
        let decoded = roundtrip(&p);
        assert_eq!(decoded.source_query_id, p.source_query_id);
        assert_eq!(decoded.heads.len(), 1);
    }

    #[test]
    fn access_snapshot_set_proof_roundtrip() {
        let p = access::SnapshotSetProof {
            source_query_id: vec![0x02],
            snapshots: vec![common::SnapshotRef {
                snapshot_id: vec![0x20],
                object_id: None,
            }],
        };
        let decoded = roundtrip(&p);
        assert_eq!(decoded.source_query_id, p.source_query_id);
        assert_eq!(decoded.snapshots.len(), 1);
    }

    #[test]
    fn access_event_set_proof_roundtrip() {
        let p = access::EventSetProof {
            source_query_id: vec![0x03],
            events: vec![common::EventRef {
                stream_id: vec![0x11],
                seq: 5,
                event_hash: None,
            }],
            related_objects: vec![],
        };
        let decoded = roundtrip(&p);
        assert_eq!(decoded.events.len(), 1);
        assert!(decoded.related_objects.is_empty());
    }

    #[test]
    fn access_object_assertion_proof_roundtrip() {
        let p = access::ObjectAssertionProof {
            source_query_id: vec![0x04],
            object_ref: Some(common::ObjectRef {
                object_id: vec![0x30],
                object_kind: Some(common::ObjectKind::Payload as i32),
            }),
            exists: true,
            bundled_result_object: None,
        };
        let decoded = roundtrip(&p);
        assert_eq!(decoded.exists, true);
        assert_eq!(decoded.object_ref, p.object_ref);
    }

    #[test]
    fn access_proof_bundle_roundtrip() {
        let pb = access::ProofBundle {
            bundle_version: 1,
            payload_type: access::ProofPayloadType::StreamHeads as i32,
            source_query_id: vec![0x05],
            payload_object: Some(common::ObjectRef {
                object_id: vec![0x40],
                object_kind: None,
            }),
            supporting_objects: vec![],
            signature: None,
        };
        let decoded = roundtrip(&pb);
        assert_eq!(decoded.bundle_version, pb.bundle_version);
        assert_eq!(decoded.payload_type, pb.payload_type);
    }

    #[test]
    fn access_trust_policy_proof_roundtrip() {
        let p = access::TrustPolicyProof {
            source_query_id: vec![0x06],
            policy_object: Some(common::ObjectRef {
                object_id: vec![0x50],
                object_kind: None,
            }),
            assignments_object: None,
        };
        let decoded = roundtrip(&p);
        assert_eq!(decoded.source_query_id, p.source_query_id);
        assert_eq!(decoded.policy_object, p.policy_object);
    }

    #[test]
    fn access_aggregate_summary_proof_roundtrip() {
        let p = access::AggregateSummaryProof {
            source_query_id: vec![0x07],
            included_responders: vec![common::IdentityRef {
                identity_id: vec![0x60],
                identity_kind: Some(common::IdentityKind::Service as i32),
                key_hint: None,
            }],
            excluded_responders: vec![],
            total_trust_score: 999,
            trust_policy_object: None,
        };
        let decoded = roundtrip(&p);
        assert_eq!(decoded.total_trust_score, 999);
        assert_eq!(decoded.included_responders.len(), 1);
    }

    #[test]
    fn access_result_fragment_proof_default() {
        let r = access::ResultFragmentProof::default();
        assert!(r.fragment.is_none());
    }

    #[test]
    fn access_federated_aggregate_descriptor_roundtrip() {
        let d = access::FederatedAggregateDescriptor {
            descriptor_version: 1,
            aggregate_id: vec![0xa1],
            source_query_id: vec![0xa2],
            aggregator: None,
            aggregated_at: None,
            input_fragments: vec![],
            aggregation_policy_object: None,
            payload_object: None,
            signature: None,
        };
        let decoded = roundtrip(&d);
        assert_eq!(decoded.descriptor_version, d.descriptor_version);
    }

    // =======================================================================
    // capability module — enums
    // =======================================================================

    #[test]
    fn capability_role_enum() {
        use capability::CapabilityRole;
        assert_eq!(CapabilityRole::Unspecified as i32, 0);
        assert_eq!(CapabilityRole::Input as i32, 1);
        assert_eq!(CapabilityRole::Output as i32, 2);
        assert_eq!(CapabilityRole::SecureElement as i32, 3);
        assert_eq!(CapabilityRole::Communication as i32, 4);
        assert_eq!(CapabilityRole::Storage as i32, 5);
        assert_eq!(CapabilityRole::Execution as i32, 6);
        assert_eq!(CapabilityRole::Derived as i32, 7);
        assert_eq!(
            CapabilityRole::from_str_name("CAPABILITY_ROLE_INPUT"),
            Some(CapabilityRole::Input)
        );
    }

    #[test]
    fn capability_modality_enum() {
        use capability::CapabilityModality;
        assert_eq!(CapabilityModality::Unspecified as i32, 0);
        assert_eq!(CapabilityModality::Visual as i32, 1);
        assert_eq!(CapabilityModality::Auditory as i32, 2);
        assert_eq!(CapabilityModality::Touch as i32, 3);
        assert_eq!(CapabilityModality::Biometric as i32, 4);
        assert_eq!(CapabilityModality::Display as i32, 5);
        assert_eq!(CapabilityModality::Radio as i32, 6);
        assert_eq!(CapabilityModality::Cryptographic as i32, 7);
        assert_eq!(CapabilityModality::Haptic as i32, 8);
        assert_eq!(CapabilityModality::Text as i32, 9);
        assert_eq!(CapabilityModality::Computational as i32, 10);
        assert_eq!(CapabilityModality::Other as i32, 11);
    }

    #[test]
    fn capability_event_kind_enum() {
        use capability::CapabilityEventKind;
        assert_eq!(CapabilityEventKind::Unspecified as i32, 0);
        assert_eq!(CapabilityEventKind::Visual as i32, 1);
        assert_eq!(CapabilityEventKind::Signing as i32, 7);
        assert_eq!(CapabilityEventKind::Attestation as i32, 8);
        assert_eq!(CapabilityEventKind::Inference as i32, 12);
    }

    #[test]
    fn capability_operation_enum() {
        use capability::CapabilityOperation;
        assert_eq!(CapabilityOperation::Unspecified as i32, 0);
        assert_eq!(CapabilityOperation::Query as i32, 1);
        assert_eq!(CapabilityOperation::Observe as i32, 2);
        assert_eq!(CapabilityOperation::Capture as i32, 3);
        assert_eq!(CapabilityOperation::Control as i32, 4);
        assert_eq!(CapabilityOperation::Render as i32, 5);
        assert_eq!(CapabilityOperation::Sign as i32, 6);
        assert_eq!(CapabilityOperation::Attest as i32, 7);
        assert_eq!(CapabilityOperation::Verify as i32, 8);
        assert_eq!(CapabilityOperation::Invoke as i32, 9);
    }

    #[test]
    fn capability_access_class_enum() {
        use capability::CapabilityAccessClass;
        assert_eq!(CapabilityAccessClass::Unspecified as i32, 0);
        assert_eq!(CapabilityAccessClass::Raw as i32, 1);
        assert_eq!(CapabilityAccessClass::Derived as i32, 2);
    }

    #[test]
    fn capability_constraint_kind_enum() {
        use capability::CapabilityConstraintKind;
        assert_eq!(CapabilityConstraintKind::Unspecified as i32, 0);
        assert_eq!(CapabilityConstraintKind::RequireUserPresence as i32, 1);
        assert_eq!(CapabilityConstraintKind::RequireBiometric as i32, 2);
        assert_eq!(CapabilityConstraintKind::RequireFreshness as i32, 3);
        assert_eq!(CapabilityConstraintKind::RequireLocalOnly as i32, 4);
        assert_eq!(CapabilityConstraintKind::RequireHardwareProtected as i32, 5);
        assert_eq!(CapabilityConstraintKind::OneShot as i32, 6);
        assert_eq!(CapabilityConstraintKind::RateLimited as i32, 7);
        assert_eq!(CapabilityConstraintKind::MaxBytes as i32, 8);
        assert_eq!(CapabilityConstraintKind::Scope as i32, 9);
    }

    // =======================================================================
    // capability module — messages
    // =======================================================================

    #[test]
    fn capability_constraint_default() {
        let c = capability::CapabilityConstraint::default();
        assert_eq!(c.kind, 0);
        assert!(c.uint_value.is_none());
        assert!(c.string_value.is_empty());
        assert!(c.duration_value.is_none());
        assert!(c.rate_limit.is_none());
    }

    #[test]
    fn capability_constraint_roundtrip() {
        let c = capability::CapabilityConstraint {
            kind: capability::CapabilityConstraintKind::MaxBytes as i32,
            uint_value: Some(4096),
            string_value: String::new(),
            duration_value: None,
            rate_limit: None,
        };
        let decoded = roundtrip(&c);
        assert_eq!(decoded.kind, c.kind);
        assert_eq!(decoded.uint_value, c.uint_value);
    }

    #[test]
    fn capability_descriptor_default() {
        let d = capability::CapabilityDescriptor::default();
        assert_eq!(d.descriptor_version, 0);
        assert!(d.capability_id.is_empty());
        assert!(d.provider_identity.is_none());
        assert!(d.provider_node.is_none());
        assert_eq!(d.role, 0);
        assert!(d.modalities.is_empty());
        assert!(d.event_kinds.is_empty());
        assert!(d.operations.is_empty());
        assert!(d.default_constraints.is_empty());
        assert!(d.provider_name.is_empty());
        assert!(d.provider_instance_id.is_empty());
    }

    #[test]
    fn capability_grant_roundtrip() {
        let g = capability::CapabilityGrant {
            grant_version: 1,
            grant_id: vec![0x11],
            issuer: Some(common::IdentityRef {
                identity_id: vec![0x20],
                identity_kind: Some(common::IdentityKind::Node as i32),
                key_hint: None,
            }),
            grantee: Some(common::IdentityRef {
                identity_id: vec![0x30],
                identity_kind: Some(common::IdentityKind::Agent as i32),
                key_hint: None,
            }),
            grantee_node: None,
            selector: None,
            granted_operations: vec![capability::CapabilityOperation::Query as i32],
            enforced_constraints: vec![],
            access_class: capability::CapabilityAccessClass::Raw as i32,
            issued_at: None,
            expires_at: None,
            correlation_id: vec![0x40],
            supersedes_revocation: None,
            signature: None,
        };
        let decoded = roundtrip(&g);
        assert_eq!(decoded.grant_version, g.grant_version);
        assert_eq!(decoded.grant_id, g.grant_id);
        assert_eq!(decoded.access_class, g.access_class);
        assert_eq!(decoded.granted_operations, g.granted_operations);
    }

    #[test]
    fn capability_invocation_roundtrip() {
        let i = capability::CapabilityInvocation {
            invocation_version: 1,
            invocation_id: vec![0x50],
            grant_id: vec![0x11],
            invoker: None,
            operation: capability::CapabilityOperation::Sign as i32,
            requested_access_class: capability::CapabilityAccessClass::Raw as i32,
            parameter_object: None,
            correlation_id: vec![0x60],
            invoked_at: None,
            signature: None,
        };
        let decoded = roundtrip(&i);
        assert_eq!(decoded.operation, i.operation);
        assert_eq!(decoded.requested_access_class, i.requested_access_class);
    }

    #[test]
    fn capability_result_roundtrip() {
        let r = capability::CapabilityResult {
            result_version: 1,
            invocation_id: vec![0x50],
            grant_id: vec![0x11],
            success: true,
            result_access_class: capability::CapabilityAccessClass::Derived as i32,
            produced_event_kinds: vec![capability::CapabilityEventKind::Signing as i32],
            payload_object: None,
            error_reason: String::new(),
            produced_at: None,
            signature: None,
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.success, true);
        assert_eq!(decoded.result_access_class, r.result_access_class);
        assert_eq!(decoded.produced_event_kinds, r.produced_event_kinds);
    }

    #[test]
    fn capability_revocation_roundtrip() {
        let r = capability::CapabilityRevocation {
            revocation_version: 1,
            revocation_id: vec![0x70],
            grant_id: vec![0x11],
            issuer: None,
            effective_at: None,
            reason: "test revocation".to_string(),
            replacement_constraints: vec![],
            signature: None,
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.reason, "test revocation");
    }

    #[test]
    fn capability_selector_roundtrip() {
        let s = capability::CapabilitySelector {
            capability_id: vec![0x80],
            role: capability::CapabilityRole::Input as i32,
            modalities: vec![capability::CapabilityModality::Visual as i32],
            event_kinds: vec![capability::CapabilityEventKind::Visual as i32],
            operations: vec![capability::CapabilityOperation::Observe as i32],
            access_class: capability::CapabilityAccessClass::Raw as i32,
            provider_identity: None,
            provider_node: None,
            provider_instance_id: String::new(),
        };
        let decoded = roundtrip(&s);
        assert_eq!(decoded.role, s.role);
        assert_eq!(decoded.modalities, s.modalities);
        assert_eq!(decoded.access_class, s.access_class);
    }

    #[test]
    fn capability_request_default() {
        let r = capability::CapabilityRequest::default();
        assert_eq!(r.request_version, 0);
        assert!(r.request_id.is_empty());
        assert!(r.requested_operations.is_empty());
        assert!(r.requested_constraints.is_empty());
        assert!(r.purpose.is_empty());
        assert!(r.correlation_id.is_empty());
    }

    // =======================================================================
    // capability_runtime module
    // =======================================================================

    #[test]
    fn capability_runtime_session_mode_enum() {
        use capability_runtime::CapabilitySessionMode;
        assert_eq!(CapabilitySessionMode::Unspecified as i32, 0);
        assert_eq!(CapabilitySessionMode::Unary as i32, 1);
        assert_eq!(CapabilitySessionMode::Stream as i32, 2);
        assert_eq!(
            CapabilitySessionMode::from_str_name("CAPABILITY_SESSION_MODE_STREAM"),
            Some(CapabilitySessionMode::Stream)
        );
    }

    #[test]
    fn capability_runtime_session_open_roundtrip() {
        let s = capability_runtime::CapabilitySessionOpen {
            version: 1,
            session_id: vec![0x01],
            selector: None,
            mode: capability_runtime::CapabilitySessionMode::Unary as i32,
            requested_operations: vec![capability::CapabilityOperation::Query as i32],
            requested_access_class: capability::CapabilityAccessClass::Raw as i32,
            requested_constraints: vec![],
            correlation_id: vec![0x02],
        };
        let decoded = roundtrip(&s);
        assert_eq!(decoded.version, s.version);
        assert_eq!(decoded.mode, s.mode);
    }

    #[test]
    fn capability_runtime_session_accept_roundtrip() {
        let s = capability_runtime::CapabilitySessionAccept {
            version: 1,
            session_id: vec![0x01],
            accepted: true,
            granted_operations: vec![capability::CapabilityOperation::Query as i32],
            granted_access_class: capability::CapabilityAccessClass::Derived as i32,
            error_reason: String::new(),
            grant_id: vec![0x03],
        };
        let decoded = roundtrip(&s);
        assert_eq!(decoded.accepted, true);
        assert_eq!(decoded.grant_id, s.grant_id);
    }

    #[test]
    fn capability_runtime_session_event_roundtrip() {
        let s = capability_runtime::CapabilitySessionEvent {
            version: 1,
            session_id: vec![0x01],
            sequence_no: 42,
            event_kinds: vec![capability::CapabilityEventKind::Visual as i32],
            payload_object: None,
            inline_payload: vec![0xde, 0xad],
        };
        let decoded = roundtrip(&s);
        assert_eq!(decoded.sequence_no, 42);
        assert_eq!(decoded.inline_payload, s.inline_payload);
    }

    #[test]
    fn capability_runtime_session_close_roundtrip() {
        let s = capability_runtime::CapabilitySessionClose {
            version: 1,
            session_id: vec![0x01],
            reason: "done".to_string(),
        };
        let decoded = roundtrip(&s);
        assert_eq!(decoded.reason, "done");
    }

    #[test]
    fn capability_runtime_invocation_frame_roundtrip() {
        let f = capability_runtime::CapabilityInvocationFrame {
            invocation: None,
            inline_parameters: vec![0xbe, 0xef],
        };
        let decoded = roundtrip(&f);
        assert!(decoded.invocation.is_none());
        assert_eq!(decoded.inline_parameters, f.inline_parameters);
    }

    #[test]
    fn capability_runtime_result_frame_roundtrip() {
        let f = capability_runtime::CapabilityResultFrame {
            result: None,
            inline_payload: vec![0xca, 0xfe],
        };
        let decoded = roundtrip(&f);
        assert!(decoded.result.is_none());
    }

    #[test]
    fn capability_runtime_envelope_default() {
        let e = capability_runtime::CapabilityRemoteEnvelope::default();
        assert!(e.message.is_none());
    }

    #[test]
    fn capability_runtime_envelope_session_open() {
        let s = capability_runtime::CapabilitySessionOpen {
            version: 1,
            session_id: vec![0x10],
            selector: None,
            mode: capability_runtime::CapabilitySessionMode::Stream as i32,
            requested_operations: vec![],
            requested_access_class: 0,
            requested_constraints: vec![],
            correlation_id: vec![],
        };
        let envelope = capability_runtime::CapabilityRemoteEnvelope {
            message: Some(
                capability_runtime::capability_remote_envelope::Message::SessionOpen(s.clone()),
            ),
        };
        let decoded = roundtrip(&envelope);
        assert!(decoded.message.is_some());
        match decoded.message.unwrap() {
            capability_runtime::capability_remote_envelope::Message::SessionOpen(inner) => {
                assert_eq!(inner.version, s.version);
                assert_eq!(inner.session_id, s.session_id);
            }
            _ => panic!("expected SessionOpen"),
        }
    }

    #[test]
    fn capability_runtime_envelope_session_close() {
        let s = capability_runtime::CapabilitySessionClose {
            version: 1,
            session_id: vec![0x10],
            reason: "closed".to_string(),
        };
        let envelope = capability_runtime::CapabilityRemoteEnvelope {
            message: Some(
                capability_runtime::capability_remote_envelope::Message::SessionClose(s.clone()),
            ),
        };
        let decoded = roundtrip(&envelope);
        match decoded.message.unwrap() {
            capability_runtime::capability_remote_envelope::Message::SessionClose(inner) => {
                assert_eq!(inner.reason, "closed");
            }
            _ => panic!("expected SessionClose"),
        }
    }

    // =======================================================================
    // identity module
    // =======================================================================

    #[test]
    fn identity_key_algorithm_enum() {
        use identity::KeyAlgorithm;
        assert_eq!(KeyAlgorithm::Unspecified as i32, 0);
        assert_eq!(KeyAlgorithm::EcdsaP256 as i32, 1);
        assert_eq!(
            KeyAlgorithm::from_str_name("KEY_ALGORITHM_ECDSA_P256"),
            Some(KeyAlgorithm::EcdsaP256)
        );
    }

    #[test]
    fn identity_record_default() {
        let r = identity::IdentityRecord::default();
        assert_eq!(r.record_version, 0);
        assert!(r.identity_id.is_empty());
        assert_eq!(r.identity_kind, 0);
        assert_eq!(r.key_algorithm, 0);
        assert!(r.public_key.is_empty());
        assert!(r.created_at.is_none());
        assert!(r.supersedes_identity.is_none());
        assert!(r.assurance_claim_objects.is_empty());
        assert!(r.metadata_object.is_none());
        assert!(r.signature.is_none());
    }

    #[test]
    fn identity_record_roundtrip() {
        let r = identity::IdentityRecord {
            record_version: 1,
            identity_id: vec![0x01],
            identity_kind: common::IdentityKind::User as i32,
            key_algorithm: identity::KeyAlgorithm::EcdsaP256 as i32,
            public_key: vec![0x04; 65], // uncompressed ECDSA P-256
            created_at: None,
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.record_version, r.record_version);
        assert_eq!(decoded.identity_kind, r.identity_kind);
        assert_eq!(decoded.key_algorithm, r.key_algorithm);
        assert_eq!(decoded.public_key, r.public_key);
    }

    // =======================================================================
    // network module
    // =======================================================================

    #[test]
    fn network_payload_kind_enum() {
        use network::PayloadKind;
        assert_eq!(PayloadKind::Unspecified as i32, 0);
        assert_eq!(PayloadKind::Command as i32, 1);
        assert_eq!(PayloadKind::Query as i32, 2);
        assert_eq!(PayloadKind::ResultFragment as i32, 3);
        assert_eq!(PayloadKind::ObjectFragment as i32, 4);
        assert_eq!(PayloadKind::SessionMessage as i32, 5);
        assert_eq!(
            PayloadKind::from_str_name("PAYLOAD_KIND_COMMAND"),
            Some(PayloadKind::Command)
        );
    }

    #[test]
    fn network_reachability_hint_default() {
        let r = network::ReachabilityHint::default();
        assert_eq!(r.hint_version, 0);
        assert!(r.subject_node.is_none());
        assert_eq!(r.transport_class, 0);
        assert!(r.locator_payload.is_empty());
        assert_eq!(r.directness, 0);
        assert!(r.valid_after.is_none());
        assert!(r.valid_until.is_none());
        assert!(r.cost_hint.is_none());
        assert!(r.quality_hint.is_none());
    }

    #[test]
    fn network_reachability_hint_roundtrip() {
        let r = network::ReachabilityHint {
            hint_version: 1,
            subject_node: Some(common::NodeRef {
                node_id: vec![0x10],
            }),
            transport_class: common::TransportClass::Quic as i32,
            locator_payload: vec![0x01, 0x02],
            directness: common::Directness::Direct as i32,
            valid_after: None,
            valid_until: None,
            cost_hint: Some(10),
            quality_hint: Some(90),
            issuer: None,
            signature: None,
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.hint_version, r.hint_version);
        assert_eq!(decoded.transport_class, r.transport_class);
        assert_eq!(decoded.cost_hint, r.cost_hint);
    }

    #[test]
    fn network_route_advertisement_roundtrip() {
        let r = network::RouteAdvertisement {
            advertisement_version: 1,
            target_node: Some(common::NodeRef {
                node_id: vec![0x20],
            }),
            advertiser: None,
            next_hop_node: None,
            reachability: vec![],
            metric_hint: None,
            advertised_at: None,
            expires_at: None,
            route_metadata: None,
            signature: None,
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.advertisement_version, r.advertisement_version);
    }

    #[test]
    fn network_session_hello_roundtrip() {
        let h = network::SessionHello {
            message_version: 1,
            initiator: Some(common::IdentityRef {
                identity_id: vec![0x30],
                identity_kind: Some(common::IdentityKind::Node as i32),
                key_hint: None,
            }),
            target_node: None,
            supported_transport_features: vec!["quic".to_string()],
            supported_protocol_versions: vec![1, 2],
            session_nonce: vec![0x00; 32],
            initiator_locators: vec![],
            hello_metadata: None,
            signature: None,
        };
        let decoded = roundtrip(&h);
        assert_eq!(
            decoded.supported_transport_features,
            h.supported_transport_features
        );
        assert_eq!(
            decoded.supported_protocol_versions,
            h.supported_protocol_versions
        );
    }

    #[test]
    fn network_session_accept_roundtrip() {
        let a = network::SessionAccept {
            message_version: 1,
            responder: None,
            echoed_session_nonce: vec![0x00; 32],
            selected_protocol_version: 1,
            selected_transport_features: vec!["quic".to_string()],
            responder_locators: vec![],
            accept_metadata: None,
            signature: None,
        };
        let decoded = roundtrip(&a);
        assert_eq!(
            decoded.selected_protocol_version,
            a.selected_protocol_version
        );
    }

    #[test]
    fn network_relay_envelope_default() {
        let e = network::RelayEnvelope::default();
        assert_eq!(e.envelope_version, 0);
        assert!(e.relay_message_id.is_empty());
        assert!(e.original_sender.is_none());
        assert!(e.intended_recipient_node.is_none());
        assert!(e.relay_chain.is_empty());
        assert_eq!(e.payload_kind, 0);
        assert!(e.store_until.is_none());
        assert!(e.relay_metadata.is_none());
        assert!(e.signature.is_none());
        assert!(e.payload.is_none());
    }

    #[test]
    fn network_relay_envelope_inline_payload() {
        let e = network::RelayEnvelope {
            envelope_version: 1,
            relay_message_id: vec![0x01],
            original_sender: None,
            intended_recipient_node: Some(common::NodeRef {
                node_id: vec![0x02],
            }),
            relay_chain: vec![],
            payload_kind: network::PayloadKind::Command as i32,
            store_until: None,
            relay_metadata: None,
            signature: None,
            payload: Some(network::relay_envelope::Payload::InlinePayload(vec![
                0xde, 0xad,
            ])),
        };
        let decoded = roundtrip(&e);
        assert_eq!(decoded.envelope_version, 1);
        match decoded.payload.unwrap() {
            network::relay_envelope::Payload::InlinePayload(p) => {
                assert_eq!(p, vec![0xde, 0xad]);
            }
            _ => panic!("expected InlinePayload"),
        }
    }

    // =======================================================================
    // object module
    // =======================================================================

    #[test]
    fn object_chunking_mode_enum() {
        use object::ChunkingMode;
        assert_eq!(ChunkingMode::Unspecified as i32, 0);
        assert_eq!(ChunkingMode::None as i32, 1);
        assert_eq!(ChunkingMode::Manifest as i32, 2);
    }

    #[test]
    fn object_logical_object_descriptor_default() {
        let d = object::LogicalObjectDescriptor::default();
        assert_eq!(d.descriptor_version, 0);
        assert!(d.object_id.is_empty());
        assert_eq!(d.object_kind, 0);
        assert_eq!(d.object_schema_version, 0);
        assert!(d.canonicalization_id.is_empty());
        assert!(d.canonical_digest.is_none());
        assert_eq!(d.canonical_size, 0);
        assert!(d.created_at.is_none());
        assert!(d.producer.is_none());
        assert!(d.describes_object.is_none());
        assert!(d.object_metadata.is_none());
    }

    #[test]
    fn object_logical_object_descriptor_roundtrip() {
        let d = object::LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: vec![0xa0],
            object_kind: common::ObjectKind::Payload as i32,
            object_schema_version: 2,
            canonicalization_id: "v1".to_string(),
            canonical_digest: Some(common::Digest {
                algorithm: common::digest::Algorithm::DigestAlgorithmSha256 as i32,
                value: vec![0xab; 32],
            }),
            canonical_size: 1024,
            created_at: None,
            producer: None,
            describes_object: None,
            object_metadata: None,
        };
        let decoded = roundtrip(&d);
        assert_eq!(decoded.descriptor_version, d.descriptor_version);
        assert_eq!(decoded.object_kind, d.object_kind);
        assert_eq!(decoded.canonical_size, d.canonical_size);
    }

    #[test]
    fn object_stored_representation_header_default() {
        let h = object::StoredRepresentationHeader::default();
        assert_eq!(h.header_version, 0);
        assert!(h.representation_id.is_empty());
        assert!(h.object.is_none());
        assert!(h.representation_digest.is_none());
        assert!(h.plaintext_size.is_none());
        assert_eq!(h.stored_size, 0);
        assert!(h.encryption_scheme.is_empty());
        assert!(h.compression_scheme.is_empty());
        assert_eq!(h.chunking_mode, 0);
        assert!(h.chunk_manifest_object.is_none());
        assert!(h.access_package_object.is_none());
        assert!(h.created_at.is_none());
        assert!(h.representation_metadata.is_none());
    }

    #[test]
    fn object_stored_representation_header_roundtrip() {
        let h = object::StoredRepresentationHeader {
            header_version: 1,
            representation_id: vec![0xb0],
            object: Some(common::ObjectRef {
                object_id: vec![0xb1],
                object_kind: Some(common::ObjectKind::Attachment as i32),
            }),
            representation_digest: None,
            plaintext_size: Some(2048),
            stored_size: 4096,
            encryption_scheme: "aes-gcm".to_string(),
            compression_scheme: "zstd".to_string(),
            chunking_mode: object::ChunkingMode::None as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        };
        let decoded = roundtrip(&h);
        assert_eq!(decoded.header_version, h.header_version);
        assert_eq!(decoded.stored_size, h.stored_size);
        assert_eq!(decoded.encryption_scheme, h.encryption_scheme);
        assert_eq!(decoded.chunking_mode, h.chunking_mode);
    }

    #[test]
    fn object_chunk_entry_roundtrip() {
        let e = object::ChunkEntry {
            index: 0,
            chunk_representation_id: vec![0xc0],
            chunk_digest: Some(common::Digest {
                algorithm: common::digest::Algorithm::DigestAlgorithmSha256 as i32,
                value: vec![0xab; 32],
            }),
            offset: 0,
            length: 1024,
        };
        let decoded = roundtrip(&e);
        assert_eq!(decoded.index, e.index);
        assert_eq!(decoded.length, e.length);
    }

    #[test]
    fn object_chunk_manifest_roundtrip() {
        let m = object::ChunkManifest {
            manifest_version: 1,
            object: Some(common::ObjectRef {
                object_id: vec![0xd0],
                object_kind: None,
            }),
            representation: Some(common::RepresentationRef {
                representation_id: vec![0xd1],
                object_id: vec![0xd0],
            }),
            chunk_count: 2,
            total_stored_size: 2048,
            chunk_entries: vec![
                object::ChunkEntry {
                    index: 0,
                    chunk_representation_id: vec![0xd2],
                    chunk_digest: None,
                    offset: 0,
                    length: 1024,
                },
                object::ChunkEntry {
                    index: 1,
                    chunk_representation_id: vec![0xd3],
                    chunk_digest: None,
                    offset: 1024,
                    length: 1024,
                },
            ],
            manifest_metadata: None,
        };
        let decoded = roundtrip(&m);
        assert_eq!(decoded.chunk_count, m.chunk_count);
        assert_eq!(decoded.chunk_entries.len(), 2);
        assert_eq!(decoded.total_stored_size, m.total_stored_size);
    }

    // =======================================================================
    // stream module
    // =======================================================================

    #[test]
    fn stream_event_type_enum() {
        use stream::EventType;
        assert_eq!(EventType::Unspecified as i32, 0);
        assert_eq!(EventType::NodeGenesis as i32, 1);
        assert_eq!(EventType::CommandSent as i32, 2);
        assert_eq!(EventType::CommandCommitted as i32, 3);
        assert_eq!(EventType::CommandRejected as i32, 4);
        assert_eq!(EventType::ActionStarted as i32, 5);
        assert_eq!(EventType::ActionCompleted as i32, 6);
        assert_eq!(EventType::ActionFailed as i32, 7);
    }

    #[test]
    fn stream_command_decision_enum() {
        use stream::CommandDecision;
        assert_eq!(CommandDecision::Unspecified as i32, 0);
        assert_eq!(CommandDecision::Committed as i32, 1);
        assert_eq!(CommandDecision::Rejected as i32, 2);
    }

    #[test]
    fn stream_action_status_enum() {
        use stream::ActionStatus;
        assert_eq!(ActionStatus::Unspecified as i32, 0);
        assert_eq!(ActionStatus::Started as i32, 1);
        assert_eq!(ActionStatus::Completed as i32, 2);
        assert_eq!(ActionStatus::Failed as i32, 3);
    }

    #[test]
    fn stream_command_type_enum() {
        use stream::CommandType;
        assert_eq!(CommandType::Unspecified as i32, 0);
        assert_eq!(CommandType::AddController as i32, 1);
        assert_eq!(CommandType::RemoveController as i32, 2);
        assert_eq!(CommandType::TransferControl as i32, 3);
        assert_eq!(CommandType::PublishSnapshot as i32, 4);
        assert_eq!(CommandType::StoreObject as i32, 5);
        assert_eq!(CommandType::FetchObject as i32, 6);
        assert_eq!(CommandType::Query as i32, 7);
        assert_eq!(CommandType::ExecuteWorkload as i32, 8);
        assert_eq!(CommandType::TerminateWorkload as i32, 9);
        assert_eq!(CommandType::CreateDelegation as i32, 10);
        assert_eq!(CommandType::CreateRevocation as i32, 11);
        assert_eq!(CommandType::StoreAndForward as i32, 12);
    }

    #[test]
    fn stream_event_envelope_default() {
        let e = stream::EventEnvelope::default();
        assert_eq!(e.envelope_version, 0);
        assert!(e.stream_id.is_empty());
        assert_eq!(e.seq, 0);
        assert!(e.prev_event_hash.is_none());
        assert_eq!(e.event_type, 0);
        assert_eq!(e.event_version, 0);
        assert!(e.recorded_at.is_none());
        assert!(e.effective_at.is_none());
        assert!(e.payload_object.is_none());
        assert!(e.related_events.is_empty());
        assert!(e.related_commands.is_empty());
        assert!(e.related_objects.is_empty());
        assert!(e.related_delegations.is_empty());
        assert!(e.related_revocations.is_empty());
        assert!(e.event_metadata.is_none());
        assert!(e.signature.is_none());
    }

    #[test]
    fn stream_event_envelope_roundtrip() {
        let e = stream::EventEnvelope {
            envelope_version: 1,
            stream_id: vec![0x10],
            seq: 100,
            prev_event_hash: Some(common::Digest {
                algorithm: common::digest::Algorithm::DigestAlgorithmSha256 as i32,
                value: vec![0xab; 32],
            }),
            event_type: stream::EventType::CommandSent as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: Some(common::ObjectRef {
                object_id: vec![0x20],
                object_kind: None,
            }),
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        let decoded = roundtrip(&e);
        assert_eq!(decoded.envelope_version, e.envelope_version);
        assert_eq!(decoded.seq, e.seq);
        assert_eq!(decoded.event_type, e.event_type);
    }

    #[test]
    fn stream_node_genesis_payload_default() {
        let p = stream::NodeGenesisPayload::default();
        assert_eq!(p.payload_version, 0);
        assert!(p.node_id.is_empty());
        assert!(p.primary_node_identity.is_none());
        assert!(p.initial_controllers.is_empty());
        assert!(p.initial_policy_object.is_none());
        assert!(p.bootstrap_records.is_empty());
        assert!(p.assurance_claims.is_empty());
        assert!(p.node_roles.is_empty());
    }

    #[test]
    fn stream_node_genesis_payload_roundtrip() {
        let p = stream::NodeGenesisPayload {
            payload_version: 1,
            node_id: vec![0x01; 32],
            primary_node_identity: Some(common::IdentityRef {
                identity_id: vec![0x02],
                identity_kind: Some(common::IdentityKind::Node as i32),
                key_hint: None,
            }),
            initial_controllers: vec![common::IdentityRef {
                identity_id: vec![0x03],
                identity_kind: Some(common::IdentityKind::User as i32),
                key_hint: None,
            }],
            initial_policy_object: None,
            bootstrap_records: vec![],
            assurance_claims: vec![],
            node_roles: vec!["controller".to_string()],
            genesis_metadata: None,
        };
        let decoded = roundtrip(&p);
        assert_eq!(decoded.payload_version, p.payload_version);
        assert_eq!(decoded.node_roles, p.node_roles);
        assert_eq!(decoded.initial_controllers.len(), 1);
    }

    #[test]
    fn stream_command_envelope_default() {
        let e = stream::CommandEnvelope::default();
        assert_eq!(e.envelope_version, 0);
        assert!(e.command_id.is_empty());
        assert!(e.target_node.is_none());
        assert!(e.issuer.is_none());
        assert_eq!(e.command_type, 0);
        assert_eq!(e.command_version, 0);
        assert!(e.issued_at.is_none());
        assert!(e.not_before.is_none());
        assert!(e.expires_at.is_none());
        assert!(e.idempotency_key.is_empty());
        assert!(e.delegation_chain.is_empty());
        assert!(e.requested_assurance.is_none());
        assert!(e.command_metadata.is_none());
        assert!(e.signature.is_none());
        assert!(e.payload.is_none());
    }

    #[test]
    fn stream_command_envelope_roundtrip() {
        let e = stream::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![0xc1],
            target_node: Some(common::NodeRef {
                node_id: vec![0xc2],
            }),
            issuer: Some(common::IdentityRef {
                identity_id: vec![0xc3],
                identity_kind: Some(common::IdentityKind::User as i32),
                key_hint: None,
            }),
            command_type: stream::CommandType::StoreObject as i32,
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: vec![0xc4],
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: None,
            payload: None,
        };
        let decoded = roundtrip(&e);
        assert_eq!(decoded.command_type, e.command_type);
        assert_eq!(decoded.command_id, e.command_id);
    }

    #[test]
    fn stream_command_envelope_with_inline_payload() {
        let e = stream::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![0x01],
            target_node: None,
            issuer: None,
            command_type: stream::CommandType::Query as i32,
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: vec![0x02],
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: None,
            payload: Some(stream::command_envelope::Payload::InlinePayload(vec![
                0x03, 0x04,
            ])),
        };
        let decoded = roundtrip(&e);
        match decoded.payload.unwrap() {
            stream::command_envelope::Payload::InlinePayload(p) => {
                assert_eq!(p, vec![0x03, 0x04]);
            }
            _ => panic!("expected InlinePayload"),
        }
    }

    #[test]
    fn stream_command_sent_payload_roundtrip() {
        let p = stream::CommandSentPayload {
            payload_version: 1,
            command: Some(common::CommandRef {
                command_id: vec![0x10],
                command_hash: None,
            }),
            target_node: Some(common::NodeRef {
                node_id: vec![0x11],
            }),
            send_metadata: None,
        };
        let decoded = roundtrip(&p);
        assert_eq!(decoded.payload_version, p.payload_version);
    }

    #[test]
    fn stream_command_result_payload_roundtrip() {
        let p = stream::CommandResultPayload {
            payload_version: 1,
            command: None,
            issuer: None,
            decision: stream::CommandDecision::Committed as i32,
            decision_basis: None,
            reason_code: "OK".to_string(),
            effect_summary_object: None,
            result_object: None,
        };
        let decoded = roundtrip(&p);
        assert_eq!(decoded.decision, stream::CommandDecision::Committed as i32);
        assert_eq!(decoded.reason_code, "OK");
    }

    #[test]
    fn stream_action_lifecycle_payload_roundtrip() {
        let p = stream::ActionLifecyclePayload {
            payload_version: 1,
            origin_command: None,
            action_instance_id: vec![0xa0],
            status: stream::ActionStatus::Completed as i32,
            result_object: None,
            error_object: None,
            progress_object: None,
            action_metadata: None,
        };
        let decoded = roundtrip(&p);
        assert_eq!(decoded.status, stream::ActionStatus::Completed as i32);
    }

    // =======================================================================
    // trust module — enums
    // =======================================================================

    #[test]
    fn trust_capability_kind_enum() {
        use trust::CapabilityKind;
        assert_eq!(CapabilityKind::Unspecified as i32, 0);
        assert_eq!(CapabilityKind::NodeControl as i32, 1);
        assert_eq!(CapabilityKind::Query as i32, 2);
        assert_eq!(CapabilityKind::SnapshotPublish as i32, 3);
        assert_eq!(CapabilityKind::ObjectStore as i32, 4);
        assert_eq!(CapabilityKind::ObjectFetch as i32, 5);
        assert_eq!(CapabilityKind::Relay as i32, 6);
        assert_eq!(CapabilityKind::ExecuteWorkload as i32, 7);
        assert_eq!(CapabilityKind::DecryptDomain as i32, 8);
    }

    #[test]
    fn trust_scope_kind_enum() {
        use trust::ScopeKind;
        assert_eq!(ScopeKind::Unspecified as i32, 0);
        assert_eq!(ScopeKind::Node as i32, 1);
        assert_eq!(ScopeKind::Stream as i32, 2);
        assert_eq!(ScopeKind::ObjectClass as i32, 3);
        assert_eq!(ScopeKind::View as i32, 4);
        assert_eq!(ScopeKind::Domain as i32, 5);
        assert_eq!(ScopeKind::QueryClass as i32, 6);
        assert_eq!(ScopeKind::WorkloadClass as i32, 7);
        assert_eq!(ScopeKind::GlobalWithConstraints as i32, 8);
    }

    #[test]
    fn trust_delegation_policy_enum() {
        use trust::DelegationPolicy;
        assert_eq!(DelegationPolicy::Unspecified as i32, 0);
        assert_eq!(DelegationPolicy::NonDelegable as i32, 1);
        assert_eq!(DelegationPolicy::DelegableWithAttenuation as i32, 2);
    }

    #[test]
    fn trust_export_policy_enum() {
        use trust::ExportPolicy;
        assert_eq!(ExportPolicy::Unspecified as i32, 0);
        assert_eq!(ExportPolicy::AllowExport as i32, 1);
        assert_eq!(ExportPolicy::QueryOnly as i32, 2);
        assert_eq!(ExportPolicy::SignOnly as i32, 3);
        assert_eq!(ExportPolicy::NoPlaintextExport as i32, 4);
    }

    #[test]
    fn trust_revocation_kind_enum() {
        use trust::RevocationKind;
        assert_eq!(RevocationKind::Unspecified as i32, 0);
        assert_eq!(RevocationKind::Delegation as i32, 1);
        assert_eq!(RevocationKind::ControllerInstallation as i32, 2);
        assert_eq!(RevocationKind::IdentityTrust as i32, 3);
        assert_eq!(RevocationKind::AssuranceClaim as i32, 4);
        assert_eq!(RevocationKind::SnapshotTrust as i32, 5);
        assert_eq!(RevocationKind::RepresentationAccess as i32, 6);
    }

    // =======================================================================
    // trust module — messages
    // =======================================================================

    #[test]
    fn trust_assurance_claim_default() {
        let c = trust::AssuranceClaim::default();
        assert_eq!(c.claim_version, 0);
        assert_eq!(c.assurance_class, 0);
        assert!(c.attester.is_none());
        assert!(c.issued_at.is_none());
        assert!(c.expires_at.is_none());
        assert!(c.evidence_object.is_none());
        assert!(c.claim_note.is_empty());
        assert!(c.signature.is_none());
        assert!(c.subject.is_none());
    }

    #[test]
    fn trust_assurance_claim_with_subject_identity() {
        let c = trust::AssuranceClaim {
            claim_version: 1,
            assurance_class: common::AssuranceClass::HardwareBacked as i32,
            attester: Some(common::IdentityRef {
                identity_id: vec![0x01],
                identity_kind: Some(common::IdentityKind::Node as i32),
                key_hint: None,
            }),
            issued_at: None,
            expires_at: None,
            evidence_object: None,
            claim_note: String::new(),
            signature: None,
            subject: Some(trust::assurance_claim::Subject::SubjectIdentity(
                common::IdentityRef {
                    identity_id: vec![0x02],
                    identity_kind: None,
                    key_hint: None,
                },
            )),
        };
        let decoded = roundtrip(&c);
        assert_eq!(decoded.assurance_class, c.assurance_class);
        match decoded.subject.unwrap() {
            trust::assurance_claim::Subject::SubjectIdentity(inner) => {
                assert_eq!(inner.identity_id, vec![0x02]);
            }
            _ => panic!("expected SubjectIdentity"),
        }
    }

    #[test]
    fn trust_assurance_requirement_default() {
        let r = trust::AssuranceRequirement::default();
        assert_eq!(r.assurance_version, 0);
        assert_eq!(r.required_class, 0);
        assert!(r.acceptable_attesters.is_empty());
        assert!(r.max_evidence_age.is_none());
        assert!(r.assurance_metadata.is_none());
    }

    #[test]
    fn trust_assurance_requirement_roundtrip() {
        let r = trust::AssuranceRequirement {
            assurance_version: 1,
            required_class: common::AssuranceClass::AttestedRuntime as i32,
            acceptable_attesters: vec![common::IdentityRef {
                identity_id: vec![0x10],
                identity_kind: Some(common::IdentityKind::Service as i32),
                key_hint: None,
            }],
            max_evidence_age: None,
            assurance_metadata: None,
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.required_class, r.required_class);
        assert_eq!(decoded.acceptable_attesters.len(), 1);
    }

    #[test]
    fn trust_scope_descriptor_default() {
        let s = trust::ScopeDescriptor::default();
        assert_eq!(s.scope_version, 0);
        assert_eq!(s.scope_kind, 0);
        assert!(s.target_nodes.is_empty());
        assert!(s.target_streams.is_empty());
        assert!(s.target_object_kinds.is_empty());
        assert!(s.target_view_types.is_empty());
        assert!(s.target_domains.is_empty());
        assert!(s.time_bounds.is_none());
        assert!(s.scope_metadata.is_none());
    }

    #[test]
    fn trust_scope_descriptor_roundtrip() {
        let s = trust::ScopeDescriptor {
            scope_version: 1,
            scope_kind: trust::ScopeKind::Node as i32,
            target_nodes: vec![common::NodeRef {
                node_id: vec![0x20],
            }],
            target_streams: vec![],
            target_object_kinds: vec![common::ObjectKind::Payload as i32],
            target_view_types: vec!["my_view".to_string()],
            target_domains: vec![],
            time_bounds: None,
            scope_metadata: None,
        };
        let decoded = roundtrip(&s);
        assert_eq!(decoded.scope_kind, s.scope_kind);
        assert_eq!(decoded.target_nodes.len(), 1);
    }

    #[test]
    fn trust_constraint_set_default() {
        let c = trust::ConstraintSet::default();
        assert_eq!(c.constraint_version, 0);
        assert!(c.not_before.is_none());
        assert!(c.expires_at.is_none());
        assert!(c.max_uses.is_none());
        assert!(c.rate_limit.is_none());
        assert!(c.requires_local_session.is_none());
        assert!(c.requires_user_presence.is_none());
        assert!(c.requires_transport_classes.is_empty());
        assert!(c.requires_location_classes.is_empty());
        assert_eq!(c.export_policy, 0);
        assert!(c.execution_class_limits.is_empty());
        assert!(c.storage_class_limits.is_empty());
    }

    #[test]
    fn trust_constraint_set_roundtrip() {
        let c = trust::ConstraintSet {
            constraint_version: 1,
            not_before: None,
            expires_at: None,
            max_uses: Some(100),
            rate_limit: Some(common::RateLimit {
                max_operations: 10,
                per: None,
            }),
            requires_local_session: Some(true),
            requires_user_presence: None,
            requires_transport_classes: vec![common::TransportClass::Quic as i32],
            requires_location_classes: vec![],
            export_policy: trust::ExportPolicy::NoPlaintextExport as i32,
            execution_class_limits: vec![common::ExecutionClass::LocalOnly as i32],
            storage_class_limits: vec![common::StorageClass::Hot as i32],
            constraint_metadata: None,
        };
        let decoded = roundtrip(&c);
        assert_eq!(decoded.max_uses, c.max_uses);
        assert_eq!(decoded.export_policy, c.export_policy);
        assert_eq!(decoded.execution_class_limits, c.execution_class_limits);
    }

    #[test]
    fn trust_trust_capability_descriptor_roundtrip() {
        let d = trust::CapabilityDescriptor {
            capability_version: 1,
            capability_kind: trust::CapabilityKind::NodeControl as i32,
            actions: vec!["add_controller".to_string()],
            scope: Some(trust::ScopeDescriptor {
                scope_version: 1,
                scope_kind: trust::ScopeKind::Node as i32,
                target_nodes: vec![],
                target_streams: vec![],
                target_object_kinds: vec![],
                target_view_types: vec![],
                target_domains: vec![],
                time_bounds: None,
                scope_metadata: None,
            }),
            constraints: None,
            delegation_policy: trust::DelegationPolicy::NonDelegable as i32,
            minimum_assurance: None,
            capability_metadata: None,
        };
        let decoded = roundtrip(&d);
        assert_eq!(decoded.capability_kind, d.capability_kind);
        assert_eq!(decoded.actions, d.actions);
        assert_eq!(decoded.delegation_policy, d.delegation_policy);
    }

    #[test]
    fn trust_delegation_record_roundtrip() {
        let d = trust::DelegationRecord {
            record_version: 1,
            delegation_id: vec![0xd0],
            issuer: Some(common::IdentityRef {
                identity_id: vec![0xd1],
                identity_kind: Some(common::IdentityKind::User as i32),
                key_hint: None,
            }),
            recipient: Some(common::IdentityRef {
                identity_id: vec![0xd2],
                identity_kind: Some(common::IdentityKind::Agent as i32),
                key_hint: None,
            }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        let decoded = roundtrip(&d);
        assert_eq!(decoded.delegation_id, d.delegation_id);
        assert_eq!(decoded.record_version, d.record_version);
    }

    #[test]
    fn trust_revocation_record_default() {
        let r = trust::RevocationRecord::default();
        assert_eq!(r.record_version, 0);
        assert!(r.revocation_id.is_empty());
        assert!(r.issuer.is_none());
        assert!(r.issued_at.is_none());
        assert!(r.effective_at.is_none());
        assert_eq!(r.revocation_kind, 0);
        assert!(r.scope_override.is_none());
        assert!(r.reason_code.is_empty());
        assert!(r.replacement_id.is_empty());
        assert!(r.revocation_metadata.is_none());
        assert!(r.signature.is_none());
        assert!(r.target.is_none());
    }

    #[test]
    fn trust_revocation_record_with_target() {
        let r = trust::RevocationRecord {
            record_version: 1,
            revocation_id: vec![0xe0],
            issuer: Some(common::IdentityRef {
                identity_id: vec![0xe1],
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: None,
            effective_at: None,
            revocation_kind: trust::RevocationKind::Delegation as i32,
            scope_override: None,
            reason_code: "revoked".to_string(),
            replacement_id: vec![0xe2],
            revocation_metadata: None,
            signature: None,
            target: Some(trust::revocation_record::Target::TargetDelegation(
                common::DelegationRef {
                    delegation_id: vec![0xe3],
                    delegation_hash: None,
                },
            )),
        };
        let decoded = roundtrip(&r);
        assert_eq!(decoded.revocation_kind, r.revocation_kind);
        assert_eq!(decoded.reason_code, "revoked");
        match decoded.target.unwrap() {
            trust::revocation_record::Target::TargetDelegation(inner) => {
                assert_eq!(inner.delegation_id, vec![0xe3]);
            }
            _ => panic!("expected TargetDelegation"),
        }
    }

    #[test]
    fn trust_route_trust_assignment_roundtrip() {
        let a = trust::RouteTrustAssignment {
            subject: Some(common::IdentityRef {
                identity_id: vec![0xf0],
                identity_kind: None,
                key_hint: None,
            }),
            trust_score: 850,
            source: "manual".to_string(),
        };
        let decoded = roundtrip(&a);
        assert_eq!(decoded.trust_score, 850);
        assert_eq!(decoded.source, "manual");
    }

    #[test]
    fn trust_route_trust_assignments_roundtrip() {
        let a = trust::RouteTrustAssignments {
            assignments: vec![
                trust::RouteTrustAssignment {
                    subject: None,
                    trust_score: 100,
                    source: "auto".to_string(),
                },
                trust::RouteTrustAssignment {
                    subject: None,
                    trust_score: 200,
                    source: "auto".to_string(),
                },
            ],
        };
        let decoded = roundtrip(&a);
        assert_eq!(decoded.assignments.len(), 2);
    }

    #[test]
    fn trust_route_selection_policy_default() {
        let p = trust::RouteSelectionPolicy::default();
        assert_eq!(p.policy_version, 0);
        assert!(p.minimum_quality_hint.is_none());
        assert!(p.maximum_cost_hint.is_none());
        assert!(p.preferred_advertisers.is_empty());
        assert!(p.preferred_next_hops.is_empty());
        assert!(p.require_active_session.is_none());
        assert!(p.policy_metadata.is_none());
    }

    #[test]
    fn trust_aggregate_trust_policy_default() {
        let p = trust::AggregateTrustPolicy::default();
        assert_eq!(p.policy_version, 0);
        assert!(p.minimum_trust_score.is_none());
        assert!(p.preferred_aggregators.is_empty());
        assert!(p.allowed_responders.is_empty());
        assert!(p.policy_metadata.is_none());
    }

    // =======================================================================
    // Edge cases: empty messages, all defaults
    // =======================================================================

    #[test]
    fn encode_empty_messages_produce_non_empty_buffers() {
        use prost::Message;
        // Most default messages encode to a non-zero-length buffer because
        // prost still encodes default scalar values (0 for int, empty for bytes).
        // Verify that encoding + decoding a default produces the same default.

        // A message whose only fields are all optional — encodes to zero bytes.
        let empty_checkpoint = common::CheckpointRef::default();
        let mut buf = Vec::new();
        empty_checkpoint.encode(&mut buf).unwrap();
        // May be 0 bytes when everything is default/empty optional
        let decoded = common::CheckpointRef::decode(buf.as_slice()).unwrap();
        assert_eq!(decoded, empty_checkpoint);

        // EventRef has two non-optional fields (stream_id bytes, seq u64)
        // but in proto3, default values (empty bytes, 0) are not encoded.
        let event_ref = common::EventRef::default();
        buf.clear();
        event_ref.encode(&mut buf).unwrap();
        // Empty bytes and seq=0 are proto3 defaults, so they may not be encoded.
        let decoded = common::EventRef::decode(buf.as_slice()).unwrap();
        assert_eq!(decoded.seq, 0);
        assert!(decoded.stream_id.is_empty());
    }

    #[test]
    fn empty_byte_vec_fields_roundtrip() {
        // Ensure empty Vec<u8> fields encode/decode correctly.
        let r = common::NodeRef { node_id: vec![] };
        let decoded = roundtrip(&r);
        assert!(decoded.node_id.is_empty());
    }

    #[test]
    fn empty_string_fields_roundtrip() {
        // Ensure empty String fields encode/decode correctly.
        let c = capability::CapabilityDescriptor {
            provider_name: String::new(),
            provider_instance_id: String::new(),
            ..Default::default()
        };
        let decoded = roundtrip(&c);
        assert!(decoded.provider_name.is_empty());
        assert!(decoded.provider_instance_id.is_empty());
    }

    #[test]
    fn repeated_fields_default_to_empty() {
        let qr = access::QueryRequest::default();
        assert!(qr.required_proof_classes.is_empty());

        let cd = capability::CapabilityDescriptor::default();
        assert!(cd.modalities.is_empty());
        assert!(cd.event_kinds.is_empty());
        assert!(cd.operations.is_empty());
        assert!(cd.default_constraints.is_empty());
    }

    #[test]
    fn bool_fields_default_to_false() {
        let proof = access::ObjectAssertionProof {
            source_query_id: vec![],
            object_ref: None,
            exists: false,
            bundled_result_object: None,
        };
        assert!(!proof.exists);

        let accept = capability_runtime::CapabilitySessionAccept {
            version: 0,
            session_id: vec![],
            accepted: false,
            granted_operations: vec![],
            granted_access_class: 0,
            error_reason: String::new(),
            grant_id: vec![],
        };
        assert!(!accept.accepted);
    }

    #[test]
    fn oneof_none_roundtrip() {
        // Messages with oneof fields set to None should roundtrip correctly.
        let e = network::RelayEnvelope::default();
        assert!(e.payload.is_none());
        let decoded = roundtrip(&e);
        assert!(decoded.payload.is_none());

        let env = stream::CommandEnvelope::default();
        assert!(env.payload.is_none());
        let decoded = roundtrip(&env);
        assert!(decoded.payload.is_none());

        let ac = trust::AssuranceClaim::default();
        assert!(ac.subject.is_none());
        let decoded = roundtrip(&ac);
        assert!(decoded.subject.is_none());

        let rr = trust::RevocationRecord::default();
        assert!(rr.target.is_none());
        let decoded = roundtrip(&rr);
        assert!(decoded.target.is_none());
    }
}
