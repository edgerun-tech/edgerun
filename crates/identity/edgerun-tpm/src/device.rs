use crate::prelude::v1::*;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;

use crate::constants::*;
use crate::traits::{FixedTpmTransport, TpmTransport};
use crate::types::*;
use crate::wire::commands::*;
use crate::wire::parse::{
    ensure_success_response, key_info_from_read_public, parse_get_random_response,
    parse_hash_response, parse_read_public_response, parse_sign_response,
    parse_start_auth_session_response,
};
use crate::wire::{read_u16, read_u32};
use edgerun_encoding::byteorder::{read_u16_be, read_u32_be};

/// Generic TPM device wrapping any transport.
pub struct TpmDevice<T> {
    transport: T,
}

impl<T> TpmDevice<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }
}

impl<T: TpmTransport> TpmDevice<T> {
    /// Transmit a command and verify the response header.
    pub fn transmit_command(&mut self, command: &[u8]) -> Result<Vec<u8>, TpmError> {
        let response = self.transport.transact(command)?;
        ensure_success_response(&response)?;
        Ok(response)
    }

    /// Read the public area of a key.
    pub fn read_public(&mut self, handle: TpmHandle) -> Result<TpmReadPublicInfo, TpmError> {
        let response = self.transmit_command(&build_read_public_command(handle))?;
        parse_read_public_response(&response)
    }

    /// Read key metadata.
    pub fn read_key_info(&mut self, handle: TpmHandle) -> Result<TpmKeyInfo, TpmError> {
        let public = self.read_public(handle)?;
        key_info_from_read_public(handle, &public)
    }

    /// Hash data using the TPM.
    pub fn hash(&mut self, params: &TpmHashParams) -> Result<TpmHashResponse, TpmError> {
        let response = self.transmit_command(&build_hash_command(params))?;
        parse_hash_response(&response)
    }

    /// Read random bytes from the TPM RNG.
    pub fn get_random(&mut self, bytes_requested: u16) -> Result<Vec<u8>, TpmError> {
        let response = self.transmit_command(&build_get_random_command(bytes_requested))?;
        parse_get_random_response(&response)
    }

    /// Sign a pre-hashed digest (no authorization).
    pub fn sign_raw(
        &mut self,
        params: &TpmSignCommandParams,
    ) -> Result<TpmParsedSignature, TpmError> {
        let response = self.transmit_command(&build_sign_command(params))?;
        parse_sign_response(&response)
    }

    /// Sign a pre-hashed digest with auth-value authorization.
    pub fn sign_raw_with_auth_value(
        &mut self,
        params: &TpmSignCommandParams,
        auth: &TpmAuthValueSession,
    ) -> Result<TpmParsedSignature, TpmError> {
        let response = self.transmit_command(&build_sign_command_with_auth_value(params, auth))?;
        parse_sign_response(&response)
    }

    /// Start an authorization session.
    pub fn start_auth_session(
        &mut self,
        params: &TpmStartAuthSessionParams,
    ) -> Result<TpmStartAuthSessionResponse, TpmError> {
        let response = self.transmit_command(&build_start_auth_session_command(params))?;
        parse_start_auth_session_response(&response)
    }

    /// Set a PolicyCommandCode restriction on a policy session.
    pub fn policy_command_code(
        &mut self,
        session: TpmPolicySession,
        command_code: u32,
    ) -> Result<(), TpmError> {
        let response =
            self.transmit_command(&build_policy_command_code_command(session, command_code))?;
        ensure_success_response(&response)?;
        Ok(())
    }

    /// Set a PolicyPCR restriction on a policy session.
    pub fn policy_pcr(&mut self, params: &TpmPolicyPcrParams) -> Result<(), TpmError> {
        let response = self.transmit_command(&build_policy_pcr_command(params))?;
        ensure_success_response(&response)?;
        Ok(())
    }

    /// Set a PolicyAuthorize restriction on a policy session.
    pub fn policy_authorize(&mut self, params: &TpmPolicyAuthorizeParams) -> Result<(), TpmError> {
        let response = self.transmit_command(&build_policy_authorize_command(params))?;
        ensure_success_response(&response)?;
        Ok(())
    }

    /// Sign with a policy session active.
    pub fn sign_raw_with_policy_session(
        &mut self,
        params: &TpmSignCommandParams,
        session: TpmPolicySession,
    ) -> Result<TpmParsedSignature, TpmError> {
        let response =
            self.transmit_command(&build_sign_command_with_policy_session(params, session))?;
        parse_sign_response(&response)
    }

    /// Verify a signature against a public key.
    pub fn verify_signature(
        &mut self,
        key_handle: TpmHandle,
        digest: &[u8],
        signature: &TpmParsedSignature,
    ) -> Result<(), TpmError> {
        let response = self.transmit_command(&build_verify_signature_command(
            key_handle, digest, signature,
        ))?;
        ensure_success_response(&response)?;
        Ok(())
    }

    /// Send TPM2_Startup. Idempotent — ignores `TPM_RC_INITIALIZE`.
    pub fn startup(&mut self, startup_type: u16) -> Result<(), TpmError> {
        let cmd = build_startup_command(startup_type);
        let response = self.transport.transact(&cmd)?;
        let result = if response.len() < 10 {
            Err(TpmError::TpmResponseCode(0xffff_fffc))
        } else {
            let size = read_u32_be(&response, 2);
            let code = read_u32_be(&response, 6);
            if size as usize != response.len() {
                Err(TpmError::TpmResponseCode(0xffff_fffc))
            } else if code == TPM_RC_SUCCESS || code == 0x100 || code == 0x120 {
                Ok(())
            } else {
                Err(TpmError::TpmResponseCode(code))
            }
        };
        core::mem::forget(response);
        core::mem::forget(cmd);
        result
    }
}

impl<T: FixedTpmTransport + TpmTransport> TpmDevice<T> {
    fn response_code_from_fixed(&mut self, command: &[u8], response: &mut [u8]) -> u32 {
        let Ok(response_len) = self.transport.transact_into(command, response) else {
            return 0xffff_fffb;
        };

        if response_len < 10 {
            return 0xffff_fffc;
        }

        read_u32_be(response, 6)
    }

    /// Send TPM2_Startup and return only the raw response code.
    pub fn startup_response_code(&mut self, startup_type: u16) -> u32 {
        let mut cmd = [
            0x80, 0x01, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x01, 0x44, 0x00, 0x00,
        ];
        cmd[10..12].copy_from_slice(&startup_type.to_be_bytes());
        let mut response = [0u8; 32];
        self.response_code_from_fixed(&cmd, &mut response)
    }

    /// Send TPM2_ReadPublic and return only the raw response code.
    pub fn read_public_response_code(&mut self, handle: TpmHandle) -> u32 {
        let mut cmd = [
            0x80, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x00, 0x01, 0x73, 0x00, 0x00, 0x00, 0x00,
        ];
        cmd[10..14].copy_from_slice(&handle.0.to_be_bytes());
        let mut response = [0u8; 1024];
        self.response_code_from_fixed(&cmd, &mut response)
    }

    /// Send TPM2_Sign with an empty auth-value authorization session and return
    /// only the raw response code.
    pub fn sign_response_code(&mut self, handle: TpmHandle, digest: &[u8; 32]) -> u32 {
        let mut signature = [0u8; 64];
        match self.sign_p256_sha256_into(handle, digest, &mut signature) {
            Ok(_) => TPM_RC_SUCCESS,
            Err(code) => code,
        }
    }

    /// Send TPM2_Sign with an empty auth-value authorization session and write a
    /// raw P-256 ECDSA signature (`r || s`) into caller-owned storage.
    pub fn sign_p256_sha256_into(
        &mut self,
        handle: TpmHandle,
        digest: &[u8; 32],
        signature: &mut [u8; 64],
    ) -> Result<usize, u32> {
        const COMMAND_SIZE: usize = 73;
        let mut cmd = [0u8; COMMAND_SIZE];
        let mut offset = 0;

        cmd[offset..offset + 2].copy_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
        offset += 2;
        cmd[offset..offset + 4].copy_from_slice(&(COMMAND_SIZE as u32).to_be_bytes());
        offset += 4;
        cmd[offset..offset + 4].copy_from_slice(&TPM_CC_SIGN.to_be_bytes());
        offset += 4;
        cmd[offset..offset + 4].copy_from_slice(&handle.0.to_be_bytes());
        offset += 4;

        cmd[offset..offset + 4].copy_from_slice(&9u32.to_be_bytes());
        offset += 4;
        cmd[offset..offset + 4].copy_from_slice(&TPM_RS_PW.to_be_bytes());
        offset += 4;
        cmd[offset..offset + 2].copy_from_slice(&0u16.to_be_bytes());
        offset += 2;
        cmd[offset] = 0;
        offset += 1;
        cmd[offset..offset + 2].copy_from_slice(&0u16.to_be_bytes());
        offset += 2;

        cmd[offset..offset + 2].copy_from_slice(&32u16.to_be_bytes());
        offset += 2;
        cmd[offset..offset + 32].copy_from_slice(digest);
        offset += 32;
        cmd[offset..offset + 2].copy_from_slice(&TPM_ALG_ECDSA.to_be_bytes());
        offset += 2;
        cmd[offset..offset + 2].copy_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        offset += 2;
        cmd[offset..offset + 2].copy_from_slice(&TPM_ST_HASHCHECK.to_be_bytes());
        offset += 2;
        cmd[offset..offset + 4].copy_from_slice(&TPM_RH_NULL.to_be_bytes());
        offset += 4;
        cmd[offset..offset + 2].copy_from_slice(&0u16.to_be_bytes());

        let mut response = [0u8; 256];
        let Ok(response_len) = self.transport.transact_into(&cmd, &mut response) else {
            return Err(0xffff_fffb);
        };
        parse_p256_sha256_signature_response(&response[..response_len], signature)
    }

    /// Fill `out` with bytes from TPM2_GetRandom using caller-owned buffers.
    pub fn get_random_into(&mut self, out: &mut [u8]) -> usize {
        let requested = core::cmp::min(out.len(), u16::MAX as usize) as u16;
        let cmd = [
            0x80,
            0x01,
            0x00,
            0x00,
            0x00,
            0x0c,
            0x00,
            0x00,
            0x01,
            0x7b,
            (requested >> 8) as u8,
            requested as u8,
        ];
        let mut response = [0u8; 4096];
        let Ok(response_len) = self.transport.transact_into(&cmd, &mut response) else {
            return 0;
        };
        if response_len < 12 {
            return 0;
        }
        let response_code = read_u32_be(&response, 6);
        if response_code != TPM_RC_SUCCESS {
            return 0;
        }
        let random_len = read_u16_be(&response, 10) as usize;
        if 12 + random_len > response_len || random_len > out.len() {
            return 0;
        }
        out[..random_len].copy_from_slice(&response[12..12 + random_len]);
        random_len
    }

    /// Create and persist an ECDSA P-256 signing key.
    pub fn create_ecdsa_p256_signing_key(
        &mut self,
        persistent_handle: u32,
    ) -> Result<TpmProvisionedKey, TpmError> {
        let _ = self.startup(TPM_SU_CLEAR);

        let available_handle = persistent_handle;

        let object_attributes = TPMA_OBJECT_SIGN_ENCRYPT
            | TPMA_OBJECT_FIXED_TPM
            | TPMA_OBJECT_FIXED_PARENT
            | TPMA_OBJECT_SENSITIVE_DATA_ORIGIN
            | TPMA_OBJECT_USER_WITH_AUTH
            | TPMA_OBJECT_NODA;

        // TPMT_PUBLIC layout for ECDSA P-256
        let mut pub_bytes = Vec::new();
        pub_bytes.extend_from_slice(&TPM_ALG_ECC.to_be_bytes());
        pub_bytes.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        pub_bytes.extend_from_slice(&object_attributes.to_be_bytes());
        pub_bytes.extend_from_slice(&0u16.to_be_bytes());
        pub_bytes.extend_from_slice(&TPM_ALG_NULL.to_be_bytes());
        pub_bytes.extend_from_slice(&TPM_ALG_ECDSA.to_be_bytes());
        pub_bytes.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        pub_bytes.extend_from_slice(&TPM_ECC_NIST_P256.to_be_bytes());
        pub_bytes.extend_from_slice(&TPM_ALG_NULL.to_be_bytes());
        pub_bytes.extend_from_slice(&0u16.to_be_bytes());
        pub_bytes.extend_from_slice(&0u16.to_be_bytes());

        let sensitive: Vec<u8> = vec![0u8, 0];
        let outside_info: Vec<u8> = vec![0u8, 0];
        let creation_pcr: Vec<u8> = vec![0u8, 0, 0, 0];

        let auth_area = crate::wire::build_auth_command(&TpmAuthCommand {
            session_handle: TPM_RS_PW,
            nonce: vec![],
            session_attributes: 0,
            hmac: vec![],
        });
        let params_size = 2
            + pub_bytes.len() as u16
            + sensitive.len() as u16
            + outside_info.len() as u16
            + creation_pcr.len() as u16;
        let total_size = 10 + auth_area.len() + params_size as usize;

        let mut cmd = Vec::new();
        cmd.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
        cmd.extend_from_slice(&(total_size as u32).to_be_bytes());
        cmd.extend_from_slice(&TPM_CC_CREATE_PRIMARY.to_be_bytes());
        cmd.extend_from_slice(&auth_area);
        cmd.extend_from_slice(&TPM_RH_OWNER.to_be_bytes());
        cmd.extend_from_slice(&(sensitive.len() as u16).to_be_bytes());
        cmd.extend_from_slice(&sensitive);
        cmd.extend_from_slice(&(pub_bytes.len() as u16).to_be_bytes());
        cmd.extend_from_slice(&pub_bytes);
        cmd.extend_from_slice(&(outside_info.len() as u16).to_be_bytes());
        cmd.extend_from_slice(&outside_info);
        cmd.extend_from_slice(&creation_pcr);

        let response = self.transmit_command(&cmd)?;

        let mut offset = 10;
        let object_handle = read_u32(&response, &mut offset, "createPrimary:objectHandle")?;
        let out_public_size = read_u16(&response, &mut offset, "createPrimary:outPublic")? as usize;
        let out_public_start = offset;
        offset += out_public_size;

        let creation_data_size =
            read_u16(&response, &mut offset, "createPrimary:creationData")? as usize;
        offset += creation_data_size;
        let creation_hash_size =
            read_u16(&response, &mut offset, "createPrimary:creationHash")? as usize;
        offset += creation_hash_size;
        let creation_ticket_size =
            read_u16(&response, &mut offset, "createPrimary:creationTicket")? as usize;
        offset += creation_ticket_size;
        let name_size = read_u16(&response, &mut offset, "createPrimary:name")? as usize;
        let name_start = offset;

        let pub_data = &response[out_public_start..out_public_start + out_public_size];
        if pub_data.len() < 10 {
            return Err(TpmError::Protocol(
                "CreatePrimary: outPublic too short".into(),
            ));
        }
        let pub_type = read_u16_be(pub_data, 0);
        if pub_type != TPM_ALG_ECC {
            return Err(TpmError::Protocol(format!(
                "CreatePrimary returned non-ECC key (type={pub_type:#06x})"
            )));
        }
        let auth_policy_len = read_u16_be(pub_data, 8) as usize;
        if pub_data.len() < 10 + auth_policy_len {
            return Err(TpmError::Protocol(
                "CreatePrimary: authPolicy overrun".into(),
            ));
        }
        let params_offset = 10 + auth_policy_len;
        if pub_data.len() < params_offset + 10 {
            return Err(TpmError::Protocol("CreatePrimary: params too short".into()));
        }
        let unique_offset = params_offset + 10;
        if pub_data.len() < unique_offset + 2 {
            return Err(TpmError::Protocol(
                "CreatePrimary: unique.x size missing".into(),
            ));
        }
        let x_len = read_u16_be(pub_data, unique_offset) as usize;
        let x_start = unique_offset + 2;
        if pub_data.len() < x_start + 2 {
            return Err(TpmError::Protocol(
                "CreatePrimary: unique.y size missing".into(),
            ));
        }
        let y_start = x_start + x_len;
        let y_len = read_u16_be(pub_data, y_start) as usize;
        let public_key_bytes = [
            &pub_data[x_start + 2..x_start + 2 + x_len],
            &pub_data[y_start + 2..y_start + 2 + y_len],
        ]
        .concat();

        let name = response[name_start..name_start + name_size].to_vec();

        // Persist the key
        let evict_auth = crate::wire::build_auth_command(&TpmAuthCommand {
            session_handle: TPM_RS_PW,
            nonce: vec![],
            session_attributes: 0,
            hmac: vec![],
        });
        let mut evict_cmd = Vec::new();
        let evict_total = 10 + evict_auth.len() + 4 + 4;
        evict_cmd.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
        evict_cmd.extend_from_slice(&(evict_total as u32).to_be_bytes());
        evict_cmd.extend_from_slice(&TPM_CC_EVICT_CONTROL.to_be_bytes());
        evict_cmd.extend_from_slice(&evict_auth);
        evict_cmd.extend_from_slice(&TPM_RH_OWNER.to_be_bytes());
        evict_cmd.extend_from_slice(&object_handle.to_be_bytes());
        evict_cmd.extend_from_slice(&available_handle.to_be_bytes());

        self.transmit_command(&evict_cmd)?;

        // Flush transient handle
        let mut flush_cmd = Vec::new();
        flush_cmd.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        flush_cmd.extend_from_slice(&14u32.to_be_bytes());
        flush_cmd.extend_from_slice(&TPM_CC_FLUSH_CONTEXT.to_be_bytes());
        flush_cmd.extend_from_slice(&object_handle.to_be_bytes());
        let _ = self.transmit_command(&flush_cmd);

        Ok(TpmProvisionedKey {
            persistent_handle: available_handle,
            public_key_bytes,
            name,
        })
    }
}

fn parse_p256_sha256_signature_response(
    response: &[u8],
    signature: &mut [u8; 64],
) -> Result<usize, u32> {
    if response.len() < 10 {
        return Err(0xffff_fffc);
    }

    let tag = read_u16_be(response, 0);
    let size = read_u32_be(response, 2) as usize;
    let code = read_u32_be(response, 6);
    if code != TPM_RC_SUCCESS {
        return Err(code);
    }
    if size != response.len() {
        return Err(0xffff_fffc);
    }

    let mut cursor = if tag == TPM_ST_SESSIONS {
        if response.len() < 14 {
            return Err(0xffff_fffc);
        }
        14
    } else {
        10
    };

    if response.len().saturating_sub(cursor) < 4 {
        return Err(0xffff_fffc);
    }
    let scheme = read_u16_be(response, cursor);
    cursor += 2;
    let hash = read_u16_be(response, cursor);
    cursor += 2;
    if scheme != TPM_ALG_ECDSA || hash != TPM_ALG_SHA256 {
        return Err(0xffff_fffa);
    }

    let r = read_fixed_tpm2b(response, &mut cursor)?;
    let s = read_fixed_tpm2b(response, &mut cursor)?;
    if r.len() > 32 || s.len() > 32 {
        return Err(0xffff_fffa);
    }

    signature.fill(0);
    signature[32 - r.len()..32].copy_from_slice(r);
    signature[64 - s.len()..64].copy_from_slice(s);
    Ok(64)
}

fn read_fixed_tpm2b<'a>(bytes: &'a [u8], cursor: &mut usize) -> Result<&'a [u8], u32> {
    if bytes.len().saturating_sub(*cursor) < 2 {
        return Err(0xffff_fffc);
    }
    let len = read_u16_be(bytes, *cursor) as usize;
    *cursor += 2;
    if bytes.len().saturating_sub(*cursor) < len {
        return Err(0xffff_fffc);
    }
    let start = *cursor;
    *cursor += len;
    Ok(&bytes[start..start + len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fixed_p256_signature_response_pads_components() {
        let mut body = Vec::new();
        body.extend_from_slice(&TPM_ALG_ECDSA.to_be_bytes());
        body.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        body.extend_from_slice(&31u16.to_be_bytes());
        body.extend_from_slice(&[0x11; 31]);
        body.extend_from_slice(&32u16.to_be_bytes());
        body.extend_from_slice(&[0x22; 32]);

        let mut response = Vec::new();
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&body);

        let mut signature = [0u8; 64];
        assert_eq!(
            parse_p256_sha256_signature_response(&response, &mut signature),
            Ok(64)
        );
        assert_eq!(signature[0], 0);
        assert_eq!(&signature[1..32], &[0x11; 31]);
        assert_eq!(&signature[32..64], &[0x22; 32]);
    }
}
