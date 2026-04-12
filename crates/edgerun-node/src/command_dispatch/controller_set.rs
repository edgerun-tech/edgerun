use edgerun_log;
use edgerun_core::command::{command_hash, validate_command, CommandValidationContext};
use edgerun_core::protocol::{canonical_bytes, ProtocolRecord, EventEnvelope, Digest};
use edgerun_core::result::Verdict;
use edgerun_hardware_signing::MeshSigner;
use edgerun_storage::NodeStore;
use edgerun_proto::edgerun::v0::stream::{CommandDecision, CommandEnvelope, CommandResultPayload as ProtoCommandResultPayload, CommandType, EventType};
use edgerun_proto::edgerun::v0::trust::{DelegationRecord as ProtoDelegationRecord, RevocationRecord as ProtoRevocationRecord};
use edgerun_proto::edgerun::v0::common::{CommandRef, DelegationRef};
use edgerun_crypto::rand_core::RngCore;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
use prost::Message;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// Controller state
// ---------------------------------------------------------------------------

/// The current set of controller identities authorized to influence this node.
#[derive(Clone, Debug, Default)]
pub struct ControllerSet {
    controllers: HashSet<Vec<u8>>,
}

impl ControllerSet {
    pub fn new(initial: Vec<Vec<u8>>) -> Self {
        Self {
            controllers: initial.into_iter().collect(),
        }
    }

    pub fn add(&mut self, identity_id: Vec<u8>) {
        self.controllers.insert(identity_id);
    }

    pub fn remove(&mut self, identity_id: &Vec<u8>) -> bool {
        self.controllers.remove(identity_id)
    }

    pub fn contains(&self, identity_id: &Vec<u8>) -> bool {
        self.controllers.contains(identity_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.controllers.iter()
    }

    pub fn to_vec(&self) -> Vec<Vec<u8>> {
        self.controllers.iter().cloned().collect()
    }
}

