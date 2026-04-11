pub mod commands;
pub mod parse;

use crate::constants::*;
use crate::types::*;

// ---------------------------------------------------------------------------
// Shared wire encoding helpers
// ---------------------------------------------------------------------------

/// Encode a TPM2 command header into the output buffer.
pub fn encode_command_header(header: TpmCommandHeader, out: &mut Vec<u8>) {
    out.extend_from_slice(&header.tag.to_be_bytes());
    out.extend_from_slice(&header.size.to_be_bytes());
    out.extend_from_slice(&header.command_code.to_be_bytes());
}

/// Read a u16 from the buffer at the cursor position, advancing it.
pub fn read_u16(bytes: &[u8], cursor: &mut usize, label: &str) -> Result<u16, TpmError> {
    if bytes.len().saturating_sub(*cursor) < 2 {
        return Err(TpmError::Protocol(format!("missing u16 field {label}")));
    }
    let out = u16::from_be_bytes([bytes[*cursor], bytes[*cursor + 1]]);
    *cursor += 2;
    Ok(out)
}

/// Read a u32 from the buffer at the cursor position, advancing it.
pub fn read_u32(bytes: &[u8], cursor: &mut usize, label: &str) -> Result<u32, TpmError> {
    if bytes.len().saturating_sub(*cursor) < 4 {
        return Err(TpmError::Protocol(format!("missing u32 field {label}")));
    }
    let out = u32::from_be_bytes([
        bytes[*cursor],
        bytes[*cursor + 1],
        bytes[*cursor + 2],
        bytes[*cursor + 3],
    ]);
    *cursor += 4;
    Ok(out)
}

/// Read a TPM2B (length-prefixed byte array) from the buffer.
pub fn read_tpm2b(bytes: &[u8], cursor: &mut usize, label: &str) -> Result<Vec<u8>, TpmError> {
    let len = read_u16(bytes, cursor, label)? as usize;
    if bytes.len().saturating_sub(*cursor) < len {
        return Err(TpmError::Protocol(format!("{label} length exceeds buffer")));
    }
    let out = bytes[*cursor..*cursor + len].to_vec();
    *cursor += len;
    Ok(out)
}

/// Encode a `TpmNameAlgorithm` as its wire u16 value.
pub fn encode_name_algorithm(value: TpmNameAlgorithm) -> u16 {
    match value {
        TpmNameAlgorithm::Sha1 => 0x0004,
        TpmNameAlgorithm::Sha256 => 0x000B,
        TpmNameAlgorithm::Sha384 => 0x000C,
        TpmNameAlgorithm::Sha512 => 0x000D,
        TpmNameAlgorithm::Null => 0x0010,
        TpmNameAlgorithm::Unknown(v) => v,
    }
}

/// Encode a `TpmSymmetricDefinition` as its wire u16 value.
pub fn encode_symmetric_definition(value: TpmSymmetricDefinition) -> [u8; 2] {
    match value {
        TpmSymmetricDefinition::Null => 0x0010u16.to_be_bytes(),
    }
}

/// Map a wire u16 to `TpmPublicObjectType`.
pub fn map_public_object_type(value: u16) -> TpmPublicObjectType {
    match value {
        0x0001 => TpmPublicObjectType::Rsa,
        0x0008 => TpmPublicObjectType::KeyedHash,
        0x0023 => TpmPublicObjectType::Ecc,
        0x0025 => TpmPublicObjectType::SymCipher,
        other => TpmPublicObjectType::Unknown(other),
    }
}

/// Map a wire u16 to `TpmNameAlgorithm`.
pub fn map_name_algorithm(value: u16) -> TpmNameAlgorithm {
    match value {
        0x0004 => TpmNameAlgorithm::Sha1,
        0x000B => TpmNameAlgorithm::Sha256,
        0x000C => TpmNameAlgorithm::Sha384,
        0x000D => TpmNameAlgorithm::Sha512,
        0x0010 => TpmNameAlgorithm::Null,
        other => TpmNameAlgorithm::Unknown(other),
    }
}

/// Map a wire u16 to `TpmEccCurve`.
pub fn map_ecc_curve(value: u16) -> TpmEccCurve {
    match value {
        0x0003 => TpmEccCurve::NistP256,
        0x0004 => TpmEccCurve::NistP384,
        0x0040 => TpmEccCurve::Curve25519,
        other => TpmEccCurve::Unknown(other),
    }
}

/// Encode a parsed signature back to wire format.
pub fn encode_parsed_signature(signature: &TpmParsedSignature) -> Vec<u8> {
    let mut out = Vec::new();
    match signature {
        TpmParsedSignature::Rsa {
            scheme,
            hash_algorithm,
            signature,
        } => {
            out.extend_from_slice(&scheme.to_be_bytes());
            out.extend_from_slice(&encode_name_algorithm(*hash_algorithm).to_be_bytes());
            out.extend_from_slice(&(signature.len() as u16).to_be_bytes());
            out.extend_from_slice(signature);
        }
        TpmParsedSignature::Ecc {
            scheme,
            hash_algorithm,
            r,
            s,
        } => {
            out.extend_from_slice(&scheme.to_be_bytes());
            out.extend_from_slice(&encode_name_algorithm(*hash_algorithm).to_be_bytes());
            out.extend_from_slice(&(r.len() as u16).to_be_bytes());
            out.extend_from_slice(r);
            out.extend_from_slice(&(s.len() as u16).to_be_bytes());
            out.extend_from_slice(s);
        }
        TpmParsedSignature::Opaque { scheme, bytes } => {
            out.extend_from_slice(&scheme.to_be_bytes());
            out.extend_from_slice(bytes);
        }
    }
    out
}

/// Build a generic auth command area.
pub fn build_auth_command(auth: &TpmAuthCommand) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&auth.session_handle.to_be_bytes());
    out.extend_from_slice(&(auth.nonce.len() as u16).to_be_bytes());
    out.extend_from_slice(&auth.nonce);
    out.push(auth.session_attributes);
    out.extend_from_slice(&(auth.hmac.len() as u16).to_be_bytes());
    out.extend_from_slice(&auth.hmac);
    out
}

/// Build a password authorization area (no session wrapper).
pub fn build_password_auth_area(auth: &TpmPasswordAuthSession) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 2 + 1 + 2 + auth.auth_value.len());
    out.extend_from_slice(&TPM_RS_PW.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.push(auth.session_attributes);
    out.extend_from_slice(&(auth.auth_value.len() as u16).to_be_bytes());
    out.extend_from_slice(&auth.auth_value);
    out
}
