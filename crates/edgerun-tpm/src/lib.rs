//! TPM 2.0 signing backend for edgerun.
//!
//! Provides raw TPM command construction, response parsing, and a high-level
//! `TpmSigningKey` abstraction over transport-specific TPM I/O.
//!
//! ## Quick start
//!
//! ```text
//! use edgerun_tpm::{LinuxTpmSigningKey, TpmHandle};
//!
//! let key = LinuxTpmSigningKey::new("/dev/tpmrm0", TpmHandle(0x8100_0001));
//! let sig = key.sign_message(b"hello")?;
//! ```

#![no_std]

extern crate alloc;

#[cfg(all(feature = "std", not(target_os = "none")))]
extern crate std;

#[cfg(target_os = "none")]
extern crate self as std;

pub mod prelude {
    pub mod v1 {
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2024::*;
    }
}

pub mod error {
    pub use core::error::*;
}

pub mod option {
    pub use core::option::*;
}

pub mod result {
    pub use core::result::*;
}

pub mod string {
    pub use alloc::string::*;
}

pub mod vec {
    pub use alloc::vec::*;
}

// TSS2 ESAPI module removed — we now use raw TPM commands via /dev/tpmrm0

mod acpi;
mod constants;
mod crb;
mod device;
#[cfg(all(feature = "std", not(target_os = "none")))]
mod linux;
mod signing;
mod tis;
mod traits;
mod types;
mod wire;

// Explicit public API — no glob re-exports
pub use acpi::{discover_tpm2_info, parse_tpm2_table, AcpiTpm2Info};
pub use constants::{
    TPM_ALG_ECC, TPM_ALG_NULL, TPM_ALG_SHA256, TPM_CC_GET_RANDOM, TPM_CC_SIGN, TPM_ECC_NIST_P256,
    TPM_RC_SUCCESS, TPM_RH_NULL, TPM_RS_PW, TPM_ST_HASHCHECK, TPM_ST_NO_SESSIONS, TPM_SU_CLEAR,
};
pub use crb::CrbTpmTransport;
pub use device::TpmDevice;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub use linux::{LinuxTpmDevice, LinuxTpmSigningKey};
pub use signing::{
    default_sign_scheme_for_algorithm, hash_message_for_algorithm, sign_params_for_message,
    sign_prehashed_with_device, sign_record_with_tpm, sign_record_with_tpm_checked,
    signature_input_for_record, TpmTransportSigningKey,
};
pub use tis::TisTpmTransport;
pub use traits::{FixedTpmTransport, TpmSigningKey, TpmTransport};
pub use types::{
    TpmAssuranceLevel, TpmAuthCommand, TpmEccCurve, TpmError, TpmHandle, TpmHashParams, TpmKeyInfo,
    TpmNameAlgorithm, TpmParsedSignature, TpmPasswordAuthSession, TpmPolicySession,
    TpmPublicAreaInfo, TpmPublicObjectType, TpmReadPublicInfo, TpmSignCommandParams,
    TpmSignatureAlgorithm, TpmSignatureScheme,
};
pub use wire::commands::{
    build_get_random_command, build_hash_command, build_policy_authorize_command,
    build_policy_command_code_command, build_policy_pcr_command, build_read_public_command,
    build_sign_command, build_sign_command_with_password_auth,
    build_sign_command_with_policy_session, build_start_auth_session_command,
    build_startup_command, build_verify_signature_command,
};
pub use wire::parse::{
    ensure_success_response, infer_signature_algorithm, key_info_from_read_public,
    parse_get_random_response, parse_hash_response, parse_public_area, parse_read_public_response,
    parse_response_header, parse_sign_response, parse_start_auth_session_response,
};
pub use wire::{
    build_auth_command, build_password_auth_area, encode_command_header, encode_name_algorithm,
    encode_parsed_signature, encode_symmetric_definition, read_tpm2b, read_u16, read_u32,
};

// ---------------------------------------------------------------------------
// Minimal test suite
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::vec;
    use alloc::vec::Vec;

    // -----------------------------------------------------------------------
    // Wire round-trip: command builders produce correct sizes
    // -----------------------------------------------------------------------

    #[test]
    fn sign_command_no_auth_has_correct_size() {
        let params = TpmSignCommandParams {
            key_handle: TpmHandle(0x8100_0001),
            digest: vec![0u8; 32],
            scheme: TpmSignatureScheme {
                scheme: 0x0018,
                hash_algorithm: Some(TpmNameAlgorithm::Sha256),
            },
            validation_ticket_tag: TPM_ST_HASHCHECK,
            validation_ticket_hierarchy: TPM_RH_NULL,
            validation_digest: Vec::new(),
        };
        let cmd = build_sign_command(&params);
        // header(10) + handle(4) + digest_len(2) + digest(32) + scheme(2) + hash_alg(2) + ticket_tag(2) + ticket_hier(4) + ticket_digest_len(2)
        assert_eq!(cmd.len(), 10 + 4 + 2 + 32 + 2 + 2 + 2 + 4 + 2);
    }

    #[test]
    fn read_public_command_has_correct_size() {
        let cmd = build_read_public_command(TpmHandle(0x8100_0001));
        assert_eq!(cmd.len(), 14); // header(10) + handle(4)
    }

    #[test]
    fn startup_command_has_correct_size() {
        let cmd = build_startup_command(TPM_SU_CLEAR);
        assert_eq!(cmd.len(), 12); // header(10) + startup_type(2)
    }

    #[test]
    fn hash_command_has_correct_size() {
        let data = vec![0u8; 16];
        let params = TpmHashParams {
            data,
            hash_algorithm: TpmNameAlgorithm::Sha256,
            hierarchy: TPM_RH_NULL,
        };
        let cmd = build_hash_command(&params);
        assert_eq!(cmd.len(), 10 + 2 + 16 + 2 + 4); // header + len + data + alg + hierarchy
    }

    #[test]
    fn get_random_command_has_correct_size() {
        let cmd = build_get_random_command(32);
        assert_eq!(cmd.len(), 12);
        assert_eq!(&cmd[10..12], &32u16.to_be_bytes());
    }

    // -----------------------------------------------------------------------
    // Response parsing: fake success responses
    // -----------------------------------------------------------------------

    fn fake_success_response(tag: u16, body: &[u8]) -> Vec<u8> {
        let size = (10 + body.len()) as u32;
        let mut v = Vec::with_capacity(10 + body.len());
        v.extend_from_slice(&tag.to_be_bytes());
        v.extend_from_slice(&size.to_be_bytes());
        v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        v.extend_from_slice(body);
        v
    }

    #[test]
    fn parse_sign_response_ecc() {
        let mut body = Vec::new();
        body.extend_from_slice(&0x0018u16.to_be_bytes()); // scheme = ECDSA
        body.extend_from_slice(&0x000Bu16.to_be_bytes()); // hash = SHA256
        body.extend_from_slice(&32u16.to_be_bytes()); // r len
        body.extend_from_slice(&[0xAB; 32]); // r
        body.extend_from_slice(&32u16.to_be_bytes()); // s len
        body.extend_from_slice(&[0xCD; 32]); // s
        let response = fake_success_response(TPM_ST_NO_SESSIONS, &body);
        let parsed = parse_sign_response(&response).unwrap();
        match parsed {
            TpmParsedSignature::Ecc {
                scheme,
                hash_algorithm,
                r,
                s,
            } => {
                assert_eq!(scheme, 0x0018);
                assert_eq!(hash_algorithm, TpmNameAlgorithm::Sha256);
                assert_eq!(r.len(), 32);
                assert_eq!(s.len(), 32);
            }
            other => panic!("expected Ecc, got {other:?}"),
        }
    }

    #[test]
    fn parse_read_public_response_ecc() {
        // Minimal ECC TPMT_PUBLIC
        let mut pub_area = Vec::new();
        pub_area.extend_from_slice(&TPM_ALG_ECC.to_be_bytes()); // type
        pub_area.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes()); // nameAlg
        pub_area.extend_from_slice(&0x0004_0000u32.to_be_bytes()); // objectAttributes
        pub_area.extend_from_slice(&0u16.to_be_bytes()); // authPolicy size=0
        pub_area.extend_from_slice(&TPM_ALG_NULL.to_be_bytes()); // symmetric
        pub_area.extend_from_slice(&0x0010u16.to_be_bytes()); // scheme (NULL)
        pub_area.extend_from_slice(&TPM_ECC_NIST_P256.to_be_bytes()); // curveID
        pub_area.extend_from_slice(&TPM_ALG_NULL.to_be_bytes()); // KDF
        pub_area.extend_from_slice(&32u16.to_be_bytes()); // x len
        pub_area.extend_from_slice(&[0x01u8; 32]); // x
        pub_area.extend_from_slice(&32u16.to_be_bytes()); // y len
        pub_area.extend_from_slice(&[0x02u8; 32]); // y

        // name (SHA256 = 2 + 32) and qualified_name (same)
        let name = {
            let mut v = Vec::new();
            v.extend_from_slice(&0x000Bu16.to_be_bytes());
            v.extend_from_slice(&[0u8; 32]);
            v
        };
        let qname = name.clone();

        // Response body: outPublic(TPM2B = size + publicArea) + name(TPM2B) + qualifiedName(TPM2B)
        let body = {
            let mut b = Vec::new();
            b.extend_from_slice(&(pub_area.len() as u16).to_be_bytes());
            b.extend_from_slice(&pub_area);
            b.extend_from_slice(&(name.len() as u16).to_be_bytes());
            b.extend_from_slice(&name);
            b.extend_from_slice(&(qname.len() as u16).to_be_bytes());
            b.extend_from_slice(&qname);
            b
        };

        let response = fake_success_response(TPM_ST_NO_SESSIONS, &body);
        let info = parse_read_public_response(&response).unwrap();
        assert_eq!(info.public_area.object_type, TpmPublicObjectType::Ecc);
        assert_eq!(info.public_area.curve, Some(TpmEccCurve::NistP256));
        assert_eq!(info.public_area.unique.len(), 64); // x || y
    }

    // -----------------------------------------------------------------------
    // Hash response parsing
    // -----------------------------------------------------------------------

    #[test]
    fn parse_hash_response_success() {
        let mut body = Vec::new();
        body.extend_from_slice(&32u16.to_be_bytes()); // digest len
        body.extend_from_slice(&[0xAA; 32]); // digest
        body.extend_from_slice(&TPM_ST_HASHCHECK.to_be_bytes()); // ticket tag
        body.extend_from_slice(&TPM_RH_NULL.to_be_bytes()); // ticket hierarchy
        body.extend_from_slice(&0u16.to_be_bytes()); // ticket digest len (empty)
        let response = fake_success_response(TPM_ST_NO_SESSIONS, &body);
        let resp = parse_hash_response(&response).unwrap();
        assert_eq!(resp.digest.len(), 32);
        assert_eq!(resp.validation.tag, TPM_ST_HASHCHECK);
    }

    #[test]
    fn parse_get_random_response_success() {
        let mut body = Vec::new();
        body.extend_from_slice(&4u16.to_be_bytes());
        body.extend_from_slice(&[1, 2, 3, 4]);
        let response = fake_success_response(TPM_ST_NO_SESSIONS, &body);
        let random = parse_get_random_response(&response).unwrap();
        assert_eq!(random, vec![1, 2, 3, 4]);
    }

    // -----------------------------------------------------------------------
    // StartAuthSession response parsing
    // -----------------------------------------------------------------------

    #[test]
    fn parse_start_auth_session_response_success() {
        let mut body = Vec::new();
        body.extend_from_slice(&0x03000001u32.to_be_bytes()); // session handle
        body.extend_from_slice(&16u16.to_be_bytes()); // nonce_tpm len
        body.extend_from_slice(&[0u8; 16]); // nonce_tpm
        let response = fake_success_response(TPM_ST_NO_SESSIONS, &body);
        let resp = parse_start_auth_session_response(&response).unwrap();
        assert_eq!(resp.session_handle, 0x03000001);
        assert_eq!(resp.nonce_tpm.len(), 16);
    }

    // -----------------------------------------------------------------------
    // Error response handling
    // -----------------------------------------------------------------------

    #[test]
    fn ensure_success_response_detects_error() {
        let mut v = Vec::new();
        v.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        v.extend_from_slice(&10u32.to_be_bytes());
        v.extend_from_slice(&0x0000_012Fu32.to_be_bytes()); // TPM_RC_FAILURE
        assert!(ensure_success_response(&v).is_err());
    }

    #[test]
    fn ensure_success_response_detects_size_mismatch() {
        let mut v = Vec::new();
        v.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        v.extend_from_slice(&20u32.to_be_bytes()); // header says 20 but actual is 10
        v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        assert!(ensure_success_response(&v).is_err());
    }

    // -----------------------------------------------------------------------
    // Wire read helpers
    // -----------------------------------------------------------------------

    #[test]
    fn read_u16_advances_cursor() {
        let buf = [0x01, 0x02, 0x03, 0x04];
        let mut cursor = 0usize;
        assert_eq!(read_u16(&buf, &mut cursor, "test").unwrap(), 0x0102);
        assert_eq!(cursor, 2);
        assert_eq!(read_u16(&buf, &mut cursor, "test").unwrap(), 0x0304);
        assert_eq!(cursor, 4);
    }

    #[test]
    fn read_u16_fails_on_short_buffer() {
        let buf = [0x01];
        let mut cursor = 0usize;
        assert!(read_u16(&buf, &mut cursor, "test").is_err());
    }

    #[test]
    fn read_tpm2b_returns_correct_bytes() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&3u16.to_be_bytes());
        buf.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        let mut cursor = 0usize;
        let data = read_tpm2b(&buf, &mut cursor, "data").unwrap();
        assert_eq!(data, vec![0xAA, 0xBB, 0xCC]);
        assert_eq!(cursor, 5);
    }

    // -----------------------------------------------------------------------
    // Mapping functions
    // -----------------------------------------------------------------------

    #[test]
    fn map_name_algorithm_roundtrip() {
        use crate::wire::map_name_algorithm;
        let alg = TpmNameAlgorithm::Sha256;
        let encoded = encode_name_algorithm(alg);
        assert_eq!(map_name_algorithm(encoded), alg);
    }

    #[test]
    fn map_ecc_curve_known_values() {
        use crate::wire::map_ecc_curve;
        assert_eq!(map_ecc_curve(0x0003), TpmEccCurve::NistP256);
        assert_eq!(map_ecc_curve(0x0004), TpmEccCurve::NistP384);
        assert_eq!(map_ecc_curve(0x0040), TpmEccCurve::Curve25519);
        assert_eq!(map_ecc_curve(0xFFFF), TpmEccCurve::Unknown(0xFFFF));
    }

    // -----------------------------------------------------------------------
    // Signature algorithm inference
    // -----------------------------------------------------------------------

    #[test]
    fn infer_ecdsa_p256_from_public_area() {
        let pub_area = TpmPublicAreaInfo {
            object_type: TpmPublicObjectType::Ecc,
            name_algorithm: TpmNameAlgorithm::Sha256,
            object_attributes: 0,
            auth_policy: Vec::new(),
            parameters: Vec::new(),
            unique: Vec::new(),
            curve: Some(TpmEccCurve::NistP256),
            key_bits: None,
        };
        assert_eq!(
            infer_signature_algorithm(&pub_area),
            Some(TpmSignatureAlgorithm::EcdsaP256Sha256)
        );
    }

    #[test]
    fn infer_rsa_from_public_area() {
        let pub_area = TpmPublicAreaInfo {
            object_type: TpmPublicObjectType::Rsa,
            name_algorithm: TpmNameAlgorithm::Sha256,
            object_attributes: 0,
            auth_policy: Vec::new(),
            parameters: Vec::new(),
            unique: Vec::new(),
            curve: None,
            key_bits: Some(2048),
        };
        assert_eq!(
            infer_signature_algorithm(&pub_area),
            Some(TpmSignatureAlgorithm::RsaPssSha256)
        );
    }

    // -----------------------------------------------------------------------
    // Auth command builder
    // -----------------------------------------------------------------------

    #[test]
    fn build_auth_command_produces_correct_bytes() {
        let auth = TpmAuthCommand {
            session_handle: 0x03000001,
            nonce: vec![0u8; 16],
            session_attributes: 0,
            hmac: Vec::new(),
        };
        let cmd = build_auth_command(&auth);
        // handle(4) + nonce_len(2) + nonce(16) + attrs(1) + hmac_len(2)
        assert_eq!(cmd.len(), 4 + 2 + 16 + 1 + 2);
    }

    // -----------------------------------------------------------------------
    // Password auth area
    // -----------------------------------------------------------------------

    #[test]
    fn build_password_auth_area_with_empty_auth() {
        let auth = TpmPasswordAuthSession {
            auth_value: Vec::new(),
            session_attributes: 0,
        };
        let area = build_password_auth_area(&auth);
        // handle(4) + nonce(2) + attrs(1) + hmac_len(2) + hmac(0)
        assert_eq!(area.len(), 4 + 2 + 1 + 2);
        assert_eq!(&area[..4], &TPM_RS_PW.to_be_bytes());
    }

    // -----------------------------------------------------------------------
    // PolicyCommandCode command
    // -----------------------------------------------------------------------

    #[test]
    fn build_policy_command_code_correct_size() {
        let session = TpmPolicySession {
            session_handle: 0x03000001,
            session_attributes: 0,
        };
        let cmd = build_policy_command_code_command(session, TPM_CC_SIGN);
        // header(10) + session(4) + auth_len(4) + auth(9) + command_code(4)
        let auth_len = 9;
        assert_eq!(cmd.len(), 10 + 4 + 4 + auth_len + 4);
    }

    // -----------------------------------------------------------------------
    // encode_parsed_signature round-trip for ECC
    // -----------------------------------------------------------------------

    #[test]
    fn encode_parsed_signature_ecc_roundtrip() {
        let sig = TpmParsedSignature::Ecc {
            scheme: 0x0018,
            hash_algorithm: TpmNameAlgorithm::Sha256,
            r: vec![0xAB; 32],
            s: vec![0xCD; 32],
        };
        let encoded = encode_parsed_signature(&sig);
        // scheme(2) + hash_alg(2) + r_len(2) + r(32) + s_len(2) + s(32)
        assert_eq!(encoded.len(), 2 + 2 + 2 + 32 + 2 + 32);
        assert_eq!(&encoded[..2], &0x0018u16.to_be_bytes());
    }

    // -----------------------------------------------------------------------
    // key_info_from_read_public
    // -----------------------------------------------------------------------

    #[test]
    fn key_info_from_read_public_ecdsa_p256() {
        let info = TpmReadPublicInfo {
            public_area: TpmPublicAreaInfo {
                object_type: TpmPublicObjectType::Ecc,
                name_algorithm: TpmNameAlgorithm::Sha256,
                object_attributes: 0,
                auth_policy: Vec::new(),
                parameters: Vec::new(),
                unique: vec![0x01; 64], // x || y
                curve: Some(TpmEccCurve::NistP256),
                key_bits: None,
            },
            public_area_raw: Vec::new(),
            name: Vec::new(),
            qualified_name: Vec::new(),
        };
        let key_info = key_info_from_read_public(TpmHandle(0x8100_0001), &info).unwrap();
        assert_eq!(key_info.key_name, "tpm:0x81000001");
        assert_eq!(key_info.algorithm, TpmSignatureAlgorithm::EcdsaP256Sha256);
        assert_eq!(key_info.public_key.len(), 64);
    }

    // -----------------------------------------------------------------------
    // TpmError Display
    // -----------------------------------------------------------------------

    #[test]
    fn tpm_error_display_tpm_response_code() {
        let err = TpmError::TpmResponseCode(0x120);
        assert_eq!(format!("{err}"), "TPM returned response code 0x00000120");
    }

    #[test]
    fn tpm_error_display_protocol() {
        let err = TpmError::Protocol("test error".into());
        assert_eq!(format!("{err}"), "TPM protocol error: test error");
    }
}
