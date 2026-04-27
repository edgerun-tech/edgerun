use crate::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(not(target_os = "none"))]
use std::eprintln;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pubkey([u8; 32]);

impl Pubkey {
    pub const fn new_from_array(arr: [u8; 32]) -> Self {
        Self(arr)
    }

    pub const fn new(pubkey_bytes: &[u8; 32]) -> Self {
        Self(*pubkey_bytes)
    }

    pub const fn default() -> Self {
        Self([0u8; 32])
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Default for Pubkey {
    fn default() -> Self {
        Self::default()
    }
}

impl std::str::FromStr for Pubkey {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let decoded = bs58::decode(s)
            .into_vec()
            .map_err(|_| "bs58 decode failed")?;
        if decoded.len() != 32 {
            #[cfg(not(target_os = "none"))]
            eprintln!("DEBUG: decoded {} bytes from '{}'", decoded.len(), s);
            return Err("decoded key must be 32 bytes");
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&decoded);
        Ok(Self(arr))
    }
}

impl std::fmt::Display for Pubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", bs58::encode(&self.0).into_string())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AccountMeta {
    NonSigner { pubkey: Pubkey },
    Signer { pubkey: Pubkey },
}

impl AccountMeta {
    pub fn new(pubkey: Pubkey, is_signer: bool) -> Self {
        if is_signer {
            Self::Signer { pubkey }
        } else {
            Self::NonSigner { pubkey }
        }
    }

    pub fn new_readonly(pubkey: Pubkey) -> Self {
        Self::NonSigner { pubkey }
    }

    pub fn pubkey(&self) -> Pubkey {
        match self {
            Self::NonSigner { pubkey } | Self::Signer { pubkey } => *pubkey,
        }
    }

    pub fn is_signer(&self) -> bool {
        matches!(self, Self::Signer { .. })
    }
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub program_id: Pubkey,
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,
}
