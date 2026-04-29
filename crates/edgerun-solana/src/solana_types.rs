use crate::prelude::*;
use edgerun_json::{FromJson, JsonValue, JsonValueError, ToJson};

const BASE58_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl ToJson for Pubkey {
    fn to_json(&self) -> JsonValue {
        self.0.to_json()
    }
}

impl FromJson for Pubkey {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        Ok(Self(<[u8; 32]>::from_json(value)?))
    }
}

impl std::str::FromStr for Pubkey {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let decoded = base58_decode(s)?;
        if decoded.len() != 32 {
            return Err("decoded key must be 32 bytes");
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&decoded);
        Ok(Self(arr))
    }
}

impl std::fmt::Display for Pubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", base58_encode(&self.0))
    }
}

fn base58_encode(data: &[u8]) -> String {
    if data.is_empty() {
        return String::new();
    }

    let leading_zeros = data.iter().take_while(|byte| **byte == 0).count();
    let mut digits: Vec<u8> = Vec::new();

    for byte in data {
        let mut carry = *byte as u32;
        for digit in &mut digits {
            carry += (*digit as u32) << 8;
            *digit = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }

    let mut out = String::new();
    for _ in 0..leading_zeros {
        out.push('1');
    }
    for digit in digits.iter().rev() {
        out.push(BASE58_ALPHABET[*digit as usize] as char);
    }
    out
}

fn base58_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let leading_zeros = input.bytes().take_while(|byte| *byte == b'1').count();
    let mut bytes: Vec<u8> = Vec::new();

    for byte in input.bytes() {
        let value = BASE58_ALPHABET
            .iter()
            .position(|candidate| *candidate == byte)
            .ok_or("base58 decode failed")? as u32;

        let mut carry = value;
        for out_byte in &mut bytes {
            carry += (*out_byte as u32) * 58;
            *out_byte = (carry & 0xff) as u8;
            carry >>= 8;
        }
        while carry > 0 {
            bytes.push((carry & 0xff) as u8);
            carry >>= 8;
        }
    }

    let mut out = Vec::new();
    out.resize(leading_zeros, 0);
    out.extend(bytes.iter().rev());
    Ok(out)
}

#[derive(Debug, Clone, Copy)]
pub enum AccountMeta {
    NonSigner { pubkey: Pubkey },
    Signer { pubkey: Pubkey },
    Readonly { pubkey: Pubkey },
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
        Self::Readonly { pubkey }
    }

    pub fn pubkey(&self) -> Pubkey {
        match self {
            Self::NonSigner { pubkey } | Self::Signer { pubkey } | Self::Readonly { pubkey } => {
                *pubkey
            }
        }
    }

    pub fn is_signer(&self) -> bool {
        matches!(self, Self::Signer { .. })
    }

    pub fn is_readonly(&self) -> bool {
        matches!(self, Self::Readonly { .. })
    }
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub program_id: Pubkey,
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    #[test]
    fn base58_encodes_system_program() {
        assert_eq!(
            Pubkey::default().to_string(),
            "11111111111111111111111111111111"
        );
    }

    #[test]
    fn base58_roundtrips_pubkey() {
        let key = Pubkey::new_from_array([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ]);
        let encoded = key.to_string();
        assert_eq!(Pubkey::from_str(&encoded), Ok(key));
    }

    #[test]
    fn base58_rejects_invalid_character() {
        assert_eq!(
            Pubkey::from_str("O1111111111111111111111111111111"),
            Err("base58 decode failed")
        );
    }
}
