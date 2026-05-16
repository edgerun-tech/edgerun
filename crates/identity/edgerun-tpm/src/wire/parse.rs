use crate::prelude::v1::*;
use alloc::format;
use alloc::vec::Vec;
use edgerun_encoding::byteorder::{read_u16_be, read_u32_be};

use crate::constants::*;
use crate::types::*;
use crate::wire::{
    map_ecc_curve, map_name_algorithm, map_public_object_type, read_tpm2b, read_u16, read_u32,
};

type RsaParams = (Option<TpmEccCurve>, Option<u16>, Vec<u8>, Vec<u8>);
type EccParams = (Option<TpmEccCurve>, Option<u16>, Vec<u8>, Vec<u8>);

// ---------------------------------------------------------------------------
// Response parsers
// ---------------------------------------------------------------------------

/// Parse a TPM2 response header.
pub fn parse_response_header(bytes: &[u8]) -> Result<TpmResponseHeader, TpmError> {
    if bytes.len() < 10 {
        return Err(TpmError::Protocol(
            "response header shorter than 10 bytes".into(),
        ));
    }
    Ok(TpmResponseHeader {
        tag: read_u16_be(bytes, 0),
        size: read_u32_be(bytes, 2),
        response_code: read_u32_be(bytes, 6),
    })
}

/// Verify the response header indicates success and size matches.
pub fn ensure_success_response(response: &[u8]) -> Result<TpmResponseHeader, TpmError> {
    let header = parse_response_header(response)?;
    if header.size as usize != response.len() {
        return Err(TpmError::Protocol(format!(
            "response size mismatch: header={}, actual={}",
            header.size,
            response.len()
        )));
    }
    if header.response_code != TPM_RC_SUCCESS {
        return Err(TpmError::TpmResponseCode(header.response_code));
    }
    Ok(header)
}

/// Parse a TPM2_Sign response.
pub fn parse_sign_response(response: &[u8]) -> Result<TpmParsedSignature, TpmError> {
    let header = ensure_success_response(response)?;
    let mut cursor = 10usize;
    let parameter_bytes = if header.tag == TPM_ST_SESSIONS && response.len() >= 14 {
        let parameter_size = read_u32_be(response, 10) as usize;
        if 14 + parameter_size <= response.len() {
            cursor = 14;
            &response[..14 + parameter_size]
        } else {
            response
        }
    } else {
        response
    };
    let scheme = read_u16(parameter_bytes, &mut cursor, "signature_scheme")?;
    let parsed = match scheme {
        0x0014 | 0x0016 => {
            let hash_algorithm =
                map_name_algorithm(read_u16(parameter_bytes, &mut cursor, "signature_hash")?);
            let signature = read_tpm2b(parameter_bytes, &mut cursor, "rsa_signature")?;
            TpmParsedSignature::Rsa {
                scheme,
                hash_algorithm,
                signature,
            }
        }
        0x0018 | 0x001A | 0x001C => {
            let hash_algorithm =
                map_name_algorithm(read_u16(parameter_bytes, &mut cursor, "signature_hash")?);
            let r = read_tpm2b(parameter_bytes, &mut cursor, "signature_r")?;
            let s = read_tpm2b(parameter_bytes, &mut cursor, "signature_s")?;
            TpmParsedSignature::Ecc {
                scheme,
                hash_algorithm,
                r,
                s,
            }
        }
        _ => {
            let bytes = parameter_bytes[cursor..].to_vec();
            cursor = parameter_bytes.len();
            TpmParsedSignature::Opaque { scheme, bytes }
        }
    };
    if cursor != parameter_bytes.len() {
        return Err(TpmError::Protocol("trailing bytes in sign response".into()));
    }
    Ok(parsed)
}

/// Parse a TPM2_ReadPublic response.
pub fn parse_read_public_response(response: &[u8]) -> Result<TpmReadPublicInfo, TpmError> {
    let _header = ensure_success_response(response)?;
    let mut cursor = 10usize;
    let public_area_raw = read_tpm2b(response, &mut cursor, "out_public")?;
    let name = read_tpm2b(response, &mut cursor, "name")?;
    let qualified_name = read_tpm2b(response, &mut cursor, "qualified_name")?;
    if cursor != response.len() {
        return Err(TpmError::Protocol(
            "trailing bytes in read_public response".into(),
        ));
    }
    let public_area = parse_public_area(&public_area_raw)?;
    Ok(TpmReadPublicInfo {
        public_area,
        public_area_raw,
        name,
        qualified_name,
    })
}

/// Parse a TPM2_Hash response.
pub fn parse_hash_response(response: &[u8]) -> Result<TpmHashResponse, TpmError> {
    let _header = ensure_success_response(response)?;
    let mut cursor = 10usize;
    let digest = read_tpm2b(response, &mut cursor, "out_hash")?;
    let tag = read_u16(response, &mut cursor, "validation_tag")?;
    let hierarchy = read_u32(response, &mut cursor, "validation_hierarchy")?;
    let ticket_digest = read_tpm2b(response, &mut cursor, "validation_digest")?;
    if cursor != response.len() {
        return Err(TpmError::Protocol("trailing bytes in hash response".into()));
    }
    Ok(TpmHashResponse {
        digest,
        validation: TpmHashCheckTicket {
            tag,
            hierarchy,
            digest: ticket_digest,
        },
    })
}

/// Parse a TPM2_GetRandom response.
pub fn parse_get_random_response(response: &[u8]) -> Result<Vec<u8>, TpmError> {
    let _header = ensure_success_response(response)?;
    let mut cursor = 10usize;
    let random = read_tpm2b(response, &mut cursor, "random_bytes")?;
    if cursor != response.len() {
        return Err(TpmError::Protocol(
            "trailing bytes in get_random response".into(),
        ));
    }
    Ok(random)
}

/// Parse a TPM2_StartAuthSession response.
pub fn parse_start_auth_session_response(
    response: &[u8],
) -> Result<TpmStartAuthSessionResponse, TpmError> {
    let _header = ensure_success_response(response)?;
    let mut cursor = 10usize;
    let session_handle = read_u32(response, &mut cursor, "session_handle")?;
    let nonce_tpm = read_tpm2b(response, &mut cursor, "nonce_tpm")?;
    if cursor != response.len() {
        return Err(TpmError::Protocol(
            "trailing bytes in start auth session response".into(),
        ));
    }
    Ok(TpmStartAuthSessionResponse {
        session_handle,
        nonce_tpm,
    })
}

// ---------------------------------------------------------------------------
// Public area parsing
// ---------------------------------------------------------------------------

/// Infer the signature algorithm from a decoded TPMT_PUBLIC area.
pub fn infer_signature_algorithm(public_area: &TpmPublicAreaInfo) -> Option<TpmSignatureAlgorithm> {
    match (
        public_area.object_type,
        public_area.curve,
        public_area.key_bits,
    ) {
        (TpmPublicObjectType::Rsa, _, Some(_)) => Some(TpmSignatureAlgorithm::RsaPssSha256),
        (TpmPublicObjectType::Ecc, Some(TpmEccCurve::NistP256), _) => {
            Some(TpmSignatureAlgorithm::EcdsaP256Sha256)
        }
        (TpmPublicObjectType::Ecc, Some(TpmEccCurve::NistP384), _) => {
            Some(TpmSignatureAlgorithm::EcdsaP384Sha384)
        }
        (TpmPublicObjectType::Ecc, Some(TpmEccCurve::Curve25519), _) => {
            Some(TpmSignatureAlgorithm::Eddsa)
        }
        _ => None,
    }
}

/// Parse a TPMT_PUBLIC area from raw bytes.
pub fn parse_public_area(bytes: &[u8]) -> Result<TpmPublicAreaInfo, TpmError> {
    let mut cursor = 0usize;
    let object_type = map_public_object_type(read_u16(bytes, &mut cursor, "type")?);
    let name_algorithm = map_name_algorithm(read_u16(bytes, &mut cursor, "name_alg")?);
    let object_attributes = read_u32(bytes, &mut cursor, "object_attributes")?;
    let auth_policy = read_tpm2b(bytes, &mut cursor, "auth_policy")?;

    let (curve, key_bits, parameters, unique) = match object_type {
        TpmPublicObjectType::Rsa => parse_rsa_public_parameters(bytes, &mut cursor)?,
        TpmPublicObjectType::Ecc => parse_ecc_public_parameters(bytes, &mut cursor)?,
        _ => {
            let parameters = bytes[cursor..].to_vec();
            return Ok(TpmPublicAreaInfo {
                object_type,
                name_algorithm,
                object_attributes,
                auth_policy,
                parameters,
                unique: Vec::new(),
                curve: None,
                key_bits: None,
            });
        }
    };

    if cursor != bytes.len() {
        return Err(TpmError::Protocol(
            "unexpected trailing bytes in public area".into(),
        ));
    }

    Ok(TpmPublicAreaInfo {
        object_type,
        name_algorithm,
        object_attributes,
        auth_policy,
        parameters,
        unique,
        curve,
        key_bits,
    })
}

/// Derive `TpmKeyInfo` from a `TpmReadPublicInfo`.
pub fn key_info_from_read_public(
    handle: TpmHandle,
    info: &TpmReadPublicInfo,
) -> Result<TpmKeyInfo, TpmError> {
    let algorithm =
        infer_signature_algorithm(&info.public_area).ok_or(TpmError::UnsupportedPublicArea)?;
    let public_key = match info.public_area.object_type {
        TpmPublicObjectType::Rsa | TpmPublicObjectType::Ecc => info.public_area.unique.clone(),
        _ => return Err(TpmError::UnsupportedPublicArea),
    };
    Ok(TpmKeyInfo {
        key_name: format!("tpm:0x{:08x}", handle.0),
        algorithm,
        public_key,
        attestation_blob: None,
        assurance_level: TpmAssuranceLevel::Unknown,
    })
}

fn parse_rsa_public_parameters(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<(Option<TpmEccCurve>, Option<u16>, Vec<u8>, Vec<u8>), TpmError> {
    let start = *cursor;
    skip_sym_def_object(bytes, cursor)?;
    skip_rsa_scheme(bytes, cursor)?;
    let key_bits = read_u16(bytes, cursor, "rsa_key_bits")?;
    let _exponent = read_u32(bytes, cursor, "rsa_exponent")?;
    let parameters = bytes[start..*cursor].to_vec();
    let unique = read_tpm2b(bytes, cursor, "rsa_unique")?;
    Ok((None, Some(key_bits), parameters, unique))
}

fn parse_ecc_public_parameters(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<(Option<TpmEccCurve>, Option<u16>, Vec<u8>, Vec<u8>), TpmError> {
    let start = *cursor;
    skip_sym_def_object(bytes, cursor)?;
    skip_ecc_scheme(bytes, cursor)?;
    let curve = map_ecc_curve(read_u16(bytes, cursor, "ecc_curve")?);
    skip_kdf_scheme(bytes, cursor)?;
    let parameters = bytes[start..*cursor].to_vec();
    let x = read_tpm2b(bytes, cursor, "ecc_x")?;
    let y = read_tpm2b(bytes, cursor, "ecc_y")?;
    let mut unique = Vec::with_capacity(x.len() + y.len());
    unique.extend_from_slice(&x);
    unique.extend_from_slice(&y);
    Ok((Some(curve), None, parameters, unique))
}

// Skip helpers for conditional TLV fields
fn skip_sym_def_object(bytes: &[u8], cursor: &mut usize) -> Result<(), TpmError> {
    let alg = read_u16(bytes, cursor, "sym_alg")?;
    if alg != 0x0010 {
        let _key_bits = read_u16(bytes, cursor, "sym_key_bits")?;
        let _mode = read_u16(bytes, cursor, "sym_mode")?;
    }
    Ok(())
}

fn skip_rsa_scheme(bytes: &[u8], cursor: &mut usize) -> Result<(), TpmError> {
    let scheme = read_u16(bytes, cursor, "rsa_scheme")?;
    if scheme != 0x0010 {
        let _hash_alg = read_u16(bytes, cursor, "rsa_scheme_hash")?;
    }
    Ok(())
}

fn skip_ecc_scheme(bytes: &[u8], cursor: &mut usize) -> Result<(), TpmError> {
    let scheme = read_u16(bytes, cursor, "ecc_scheme")?;
    if scheme != 0x0010 {
        let _hash_alg = read_u16(bytes, cursor, "ecc_scheme_hash")?;
    }
    Ok(())
}

fn skip_kdf_scheme(bytes: &[u8], cursor: &mut usize) -> Result<(), TpmError> {
    let scheme = read_u16(bytes, cursor, "kdf_scheme")?;
    if scheme != 0x0010 {
        let _hash_alg = read_u16(bytes, cursor, "kdf_hash")?;
    }
    Ok(())
}
