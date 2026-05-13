use alloc::vec::Vec;

use crate::channel::ChannelEndpoint;
use crate::protocol::{Hash, NodeIdentity, RelayEndpoint};

pub struct PreimageBuilder {
    bytes: Vec<u8>,
}

impl PreimageBuilder {
    pub fn domain(domain: &[u8]) -> Self {
        let mut bytes = Vec::with_capacity(domain.len() + 1);
        bytes.extend_from_slice(domain);
        bytes.push(0);
        Self { bytes }
    }

    pub fn with_capacity(domain: &[u8], capacity: usize) -> Self {
        let mut bytes = Vec::with_capacity(domain.len() + 1 + capacity);
        bytes.extend_from_slice(domain);
        bytes.push(0);
        Self { bytes }
    }

    pub fn raw(mut self, value: &[u8]) -> Self {
        self.bytes.extend_from_slice(value);
        self
    }

    pub fn bytes(mut self, value: &[u8]) -> Self {
        self.bytes
            .extend_from_slice(&(value.len() as u64).to_be_bytes());
        self.bytes.extend_from_slice(value);
        self
    }

    pub fn hash(mut self, value: &Hash) -> Self {
        self.bytes.extend_from_slice(value);
        self
    }

    pub fn node_id(mut self, value: &Hash) -> Self {
        self.bytes.extend_from_slice(value);
        self
    }

    pub fn u16(mut self, value: u16) -> Self {
        self.bytes.extend_from_slice(&value.to_be_bytes());
        self
    }

    pub fn u16_list(mut self, values: &[u16]) -> Self {
        self.bytes
            .extend_from_slice(&(values.len() as u64).to_be_bytes());
        for value in values {
            self.bytes.extend_from_slice(&value.to_be_bytes());
        }
        self
    }

    pub fn hash_list(mut self, values: &[Hash]) -> Self {
        self.bytes
            .extend_from_slice(&(values.len() as u64).to_be_bytes());
        for value in values {
            self.bytes.extend_from_slice(value);
        }
        self
    }

    pub fn u64(mut self, value: u64) -> Self {
        self.bytes.extend_from_slice(&value.to_be_bytes());
        self
    }

    pub fn node(mut self, value: &NodeIdentity) -> Self {
        self.bytes.extend_from_slice(&value.node_id);
        self.bytes.extend_from_slice(&value.role.to_be_bytes());
        self.bytes.extend_from_slice(&value.public_key);
        self
    }

    pub fn relay(mut self, value: &RelayEndpoint) -> Self {
        self.bytes.extend_from_slice(&value.relay_node_id);
        self.channel(&value.channel)
    }

    pub fn channel(mut self, value: &ChannelEndpoint) -> Self {
        self.bytes.extend_from_slice(&value.channel_id);
        self.bytes.extend_from_slice(&value.kind.to_be_bytes());
        self.bytes
            .extend_from_slice(&(value.address.len() as u64).to_be_bytes());
        self.bytes.extend_from_slice(&value.address);
        self.bytes
            .extend_from_slice(&(value.label.as_bytes().len() as u64).to_be_bytes());
        self.bytes.extend_from_slice(value.label.as_bytes());
        self
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub struct HashBuilder {
    hasher: crate::blake3::Hasher,
}

impl HashBuilder {
    pub fn domain(domain: &[u8]) -> Self {
        let mut hasher = crate::blake3::Hasher::new();
        hasher.update(domain);
        hasher.update(&[0]);
        Self { hasher }
    }

    pub fn raw(mut self, value: &[u8]) -> Self {
        self.hasher.update(value);
        self
    }

    pub fn bytes(mut self, value: &[u8]) -> Self {
        self.hasher.update(&(value.len() as u64).to_be_bytes());
        self.hasher.update(value);
        self
    }

    pub fn hash(mut self, value: &Hash) -> Self {
        self.hasher.update(value);
        self
    }

    pub fn node_id(mut self, value: &Hash) -> Self {
        self.hasher.update(value);
        self
    }

    pub fn u16(mut self, value: u16) -> Self {
        self.hasher.update(&value.to_be_bytes());
        self
    }

    pub fn u16_list(mut self, values: &[u16]) -> Self {
        self.hasher.update(&(values.len() as u64).to_be_bytes());
        for value in values {
            self.hasher.update(&value.to_be_bytes());
        }
        self
    }

    pub fn hash_list(mut self, values: &[Hash]) -> Self {
        self.hasher.update(&(values.len() as u64).to_be_bytes());
        for value in values {
            self.hasher.update(value);
        }
        self
    }

    pub fn u64(mut self, value: u64) -> Self {
        self.hasher.update(&value.to_be_bytes());
        self
    }

    pub fn finish(self) -> Hash {
        *self.hasher.finalize().as_bytes()
    }
}
