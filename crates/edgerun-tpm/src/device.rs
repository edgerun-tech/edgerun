use alloc::format;
use alloc::vec;
use alloc::vec::Vec;

use crate::constants::*;
use crate::traits::TpmTransport;
use crate::types::*;
use crate::wire::commands::*;
use crate::wire::parse::{
    ensure_success_response, key_info_from_read_public, parse_hash_response,
    parse_read_public_response, parse_sign_response, parse_start_auth_session_response,
};
use crate::wire::{read_u16, read_u32};

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

    /// Sign a pre-hashed digest (no authorization).
    pub fn sign_raw(
        &mut self,
        params: &TpmSignCommandParams,
    ) -> Result<TpmParsedSignature, TpmError> {
        let response = self.transmit_command(&build_sign_command(params))?;
        parse_sign_response(&response)
    }

    /// Sign a pre-hashed digest with password authorization.
    pub fn sign_raw_with_password_auth(
        &mut self,
        params: &TpmSignCommandParams,
        auth: &TpmPasswordAuthSession,
    ) -> Result<TpmParsedSignature, TpmError> {
        let response =
            self.transmit_command(&build_sign_command_with_password_auth(params, auth))?;
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
        match self.transmit_command(&cmd) {
            Ok(_) => Ok(()),
            Err(TpmError::TpmResponseCode(0x120)) => Ok(()), // already started
            Err(e) => Err(e),
        }
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
        let pub_type = u16::from_be_bytes([pub_data[0], pub_data[1]]);
        if pub_type != TPM_ALG_ECC {
            return Err(TpmError::Protocol(format!(
                "CreatePrimary returned non-ECC key (type={pub_type:#06x})"
            )));
        }
        let auth_policy_len = u16::from_be_bytes([pub_data[8], pub_data[9]]) as usize;
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
        let x_len =
            u16::from_be_bytes([pub_data[unique_offset], pub_data[unique_offset + 1]]) as usize;
        let x_start = unique_offset + 2;
        if pub_data.len() < x_start + 2 {
            return Err(TpmError::Protocol(
                "CreatePrimary: unique.y size missing".into(),
            ));
        }
        let y_start = x_start + x_len;
        let y_len = u16::from_be_bytes([pub_data[y_start], pub_data[y_start + 1]]) as usize;
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
