use crate::prelude::v1::*;
use alloc::vec::Vec;

use crate::constants::*;
use crate::types::*;
use crate::wire::{
    build_auth_command, build_auth_value_area, encode_command_header, encode_name_algorithm,
    encode_symmetric_definition,
};

// ---------------------------------------------------------------------------
// Command builders
// ---------------------------------------------------------------------------

/// Build a TPM2_Sign command (no authorization).
pub fn build_sign_command(params: &TpmSignCommandParams) -> Vec<u8> {
    build_sign_command_body(params, None)
}

/// Build a TPM2_Sign command with auth-value authorization.
pub fn build_sign_command_with_auth_value(
    params: &TpmSignCommandParams,
    auth: &TpmAuthValueSession,
) -> Vec<u8> {
    build_sign_command_body(params, Some(auth))
}

/// Build a TPM2_Sign command with a policy session.
pub fn build_sign_command_with_policy_session(
    params: &TpmSignCommandParams,
    session: TpmPolicySession,
) -> Vec<u8> {
    let auth = build_auth_command(&TpmAuthCommand {
        session_handle: session.session_handle,
        nonce: Vec::new(),
        session_attributes: session.session_attributes,
        hmac: Vec::new(),
    });
    let mut parameters = Vec::new();
    parameters.extend_from_slice(&(params.digest.len() as u16).to_be_bytes());
    parameters.extend_from_slice(&params.digest);
    parameters.extend_from_slice(&params.scheme.scheme.to_be_bytes());
    match params.scheme.hash_algorithm {
        Some(alg) => parameters.extend_from_slice(&encode_name_algorithm(alg).to_be_bytes()),
        None => parameters.extend_from_slice(&0x0010u16.to_be_bytes()),
    }
    parameters.extend_from_slice(&params.validation_ticket_hierarchy.to_be_bytes());
    parameters.extend_from_slice(&(params.validation_digest.len() as u16).to_be_bytes());
    parameters.extend_from_slice(&params.validation_digest);

    let mut out = Vec::with_capacity(10 + 4 + 4 + auth.len() + parameters.len());
    encode_command_header(
        TpmCommandHeader {
            tag: TPM_ST_SESSIONS,
            size: (10 + 4 + 4 + auth.len() + parameters.len()) as u32,
            command_code: TPM_CC_SIGN,
        },
        &mut out,
    );
    out.extend_from_slice(&params.key_handle.0.to_be_bytes());
    out.extend_from_slice(&(auth.len() as u32).to_be_bytes());
    out.extend_from_slice(&auth);
    out.extend_from_slice(&parameters);
    out
}

/// Core sign command builder — shared by auth-value and no-auth variants.
fn build_sign_command_body(
    params: &TpmSignCommandParams,
    auth: Option<&TpmAuthValueSession>,
) -> Vec<u8> {
    let mut parameters = Vec::new();
    parameters.extend_from_slice(&params.key_handle.0.to_be_bytes());
    parameters.extend_from_slice(&(params.digest.len() as u16).to_be_bytes());
    parameters.extend_from_slice(&params.digest);
    parameters.extend_from_slice(&params.scheme.scheme.to_be_bytes());
    match params.scheme.hash_algorithm {
        Some(alg) => parameters.extend_from_slice(&encode_name_algorithm(alg).to_be_bytes()),
        None => parameters.extend_from_slice(&0x0010u16.to_be_bytes()),
    }
    parameters.extend_from_slice(&params.validation_ticket_tag.to_be_bytes());
    parameters.extend_from_slice(&params.validation_ticket_hierarchy.to_be_bytes());
    parameters.extend_from_slice(&(params.validation_digest.len() as u16).to_be_bytes());
    parameters.extend_from_slice(&params.validation_digest);

    match auth {
        None => {
            let mut out = Vec::with_capacity(10 + parameters.len());
            encode_command_header(
                TpmCommandHeader {
                    tag: TPM_ST_NO_SESSIONS,
                    size: (10 + parameters.len()) as u32,
                    command_code: TPM_CC_SIGN,
                },
                &mut out,
            );
            out.extend_from_slice(&parameters);
            out
        }
        Some(auth) => {
            let auth_area = build_auth_value_area(auth);
            let mut out = Vec::with_capacity(10 + parameters.len() + 4 + auth_area.len());
            encode_command_header(
                TpmCommandHeader {
                    tag: TPM_ST_SESSIONS,
                    size: (10 + parameters.len() + 4 + auth_area.len()) as u32,
                    command_code: TPM_CC_SIGN,
                },
                &mut out,
            );
            out.extend_from_slice(&params.key_handle.0.to_be_bytes());
            out.extend_from_slice(&auth_area);
            out.extend_from_slice(&parameters[4..]);
            out
        }
    }
}

/// Build a TPM2_Hash command.
pub fn build_hash_command(params: &TpmHashParams) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&(params.data.len() as u16).to_be_bytes());
    body.extend_from_slice(&params.data);
    body.extend_from_slice(&encode_name_algorithm(params.hash_algorithm).to_be_bytes());
    body.extend_from_slice(&params.hierarchy.to_be_bytes());

    let mut out = Vec::with_capacity(10 + body.len());
    encode_command_header(
        TpmCommandHeader {
            tag: TPM_ST_NO_SESSIONS,
            size: (10 + body.len()) as u32,
            command_code: TPM_CC_HASH,
        },
        &mut out,
    );
    out.extend_from_slice(&body);
    out
}

/// Build a TPM2_GetRandom command.
pub fn build_get_random_command(bytes_requested: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(12);
    encode_command_header(
        TpmCommandHeader {
            tag: TPM_ST_NO_SESSIONS,
            size: 12,
            command_code: TPM_CC_GET_RANDOM,
        },
        &mut out,
    );
    out.extend_from_slice(&bytes_requested.to_be_bytes());
    out
}

/// Build a TPM2_VerifySignature command.
pub fn build_verify_signature_command(
    key_handle: TpmHandle,
    digest: &[u8],
    signature: &TpmParsedSignature,
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&key_handle.0.to_be_bytes());
    body.extend_from_slice(&(digest.len() as u16).to_be_bytes());
    body.extend_from_slice(digest);
    body.extend_from_slice(&crate::wire::encode_parsed_signature(signature));

    let mut out = Vec::with_capacity(10 + body.len());
    encode_command_header(
        TpmCommandHeader {
            tag: TPM_ST_NO_SESSIONS,
            size: (10 + body.len()) as u32,
            command_code: TPM_CC_VERIFY_SIGNATURE,
        },
        &mut out,
    );
    out.extend_from_slice(&body);
    out
}

/// Build a TPM2_ReadPublic command.
pub fn build_read_public_command(handle: TpmHandle) -> Vec<u8> {
    let mut out = Vec::with_capacity(14);
    encode_command_header(
        TpmCommandHeader {
            tag: TPM_ST_NO_SESSIONS,
            size: 14,
            command_code: TPM_CC_READ_PUBLIC,
        },
        &mut out,
    );
    out.extend_from_slice(&handle.0.to_be_bytes());
    out
}

/// Build a TPM2_StartAuthSession command.
pub fn build_start_auth_session_command(params: &TpmStartAuthSessionParams) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&params.tpm_key.0.to_be_bytes());
    body.extend_from_slice(&params.bind.0.to_be_bytes());
    body.extend_from_slice(&(params.nonce_caller.len() as u16).to_be_bytes());
    body.extend_from_slice(&params.nonce_caller);
    body.push(params.session_type as u8);
    body.extend_from_slice(&encode_symmetric_definition(params.symmetric));
    body.extend_from_slice(&encode_name_algorithm(params.auth_hash).to_be_bytes());

    let mut out = Vec::with_capacity(10 + body.len());
    encode_command_header(
        TpmCommandHeader {
            tag: TPM_ST_NO_SESSIONS,
            size: (10 + body.len()) as u32,
            command_code: TPM_CC_START_AUTH_SESSION,
        },
        &mut out,
    );
    out.extend_from_slice(&body);
    out
}

/// Build a TPM2_PolicyPCR command.
pub fn build_policy_pcr_command(params: &TpmPolicyPcrParams) -> Vec<u8> {
    let auth = build_auth_command(&TpmAuthCommand {
        session_handle: params.session.session_handle,
        nonce: Vec::new(),
        session_attributes: params.session.session_attributes,
        hmac: Vec::new(),
    });
    let mut body = Vec::new();
    body.extend_from_slice(&params.session.session_handle.to_be_bytes());
    body.extend_from_slice(&(auth.len() as u32).to_be_bytes());
    body.extend_from_slice(&auth);
    body.extend_from_slice(&(params.pcr_digest.len() as u16).to_be_bytes());
    body.extend_from_slice(&params.pcr_digest);
    body.extend_from_slice(&encode_name_algorithm(params.selection.hash_algorithm).to_be_bytes());
    body.push(1);
    body.push(3);
    let mut bits = [0u8; 3];
    for pcr in &params.selection.pcrs {
        let index = *pcr as usize;
        if index < 24 {
            bits[index / 8] |= 1 << (index % 8);
        }
    }
    body.extend_from_slice(&bits);

    let mut out = Vec::with_capacity(10 + body.len());
    encode_command_header(
        TpmCommandHeader {
            tag: TPM_ST_SESSIONS,
            size: (10 + body.len()) as u32,
            command_code: TPM_CC_POLICY_PCR,
        },
        &mut out,
    );
    out.extend_from_slice(&body);
    out
}

/// Build a TPM2_PolicyAuthorize command.
pub fn build_policy_authorize_command(params: &TpmPolicyAuthorizeParams) -> Vec<u8> {
    let auth = build_auth_command(&TpmAuthCommand {
        session_handle: params.session.session_handle,
        nonce: Vec::new(),
        session_attributes: params.session.session_attributes,
        hmac: Vec::new(),
    });
    let mut body = Vec::new();
    body.extend_from_slice(&params.session.session_handle.to_be_bytes());
    body.extend_from_slice(&(auth.len() as u32).to_be_bytes());
    body.extend_from_slice(&auth);
    body.extend_from_slice(&(params.approved_policy.len() as u16).to_be_bytes());
    body.extend_from_slice(&params.approved_policy);
    body.extend_from_slice(&(params.policy_ref.len() as u16).to_be_bytes());
    body.extend_from_slice(&params.policy_ref);
    body.extend_from_slice(&(params.key_sign_name.len() as u16).to_be_bytes());
    body.extend_from_slice(&params.key_sign_name);
    body.extend_from_slice(&0x8021u16.to_be_bytes());
    body.extend_from_slice(&params.check_ticket_hierarchy.to_be_bytes());
    body.extend_from_slice(&(params.check_ticket_digest.len() as u16).to_be_bytes());
    body.extend_from_slice(&params.check_ticket_digest);

    let mut out = Vec::with_capacity(10 + body.len());
    encode_command_header(
        TpmCommandHeader {
            tag: TPM_ST_SESSIONS,
            size: (10 + body.len()) as u32,
            command_code: TPM_CC_POLICY_AUTHORIZE,
        },
        &mut out,
    );
    out.extend_from_slice(&body);
    out
}

/// Build a TPM2_PolicyCommandCode command.
pub fn build_policy_command_code_command(session: TpmPolicySession, command_code: u32) -> Vec<u8> {
    let auth = build_auth_command(&TpmAuthCommand {
        session_handle: session.session_handle,
        nonce: Vec::new(),
        session_attributes: session.session_attributes,
        hmac: Vec::new(),
    });
    let mut body = Vec::new();
    body.extend_from_slice(&session.session_handle.to_be_bytes());
    body.extend_from_slice(&(auth.len() as u32).to_be_bytes());
    body.extend_from_slice(&auth);
    body.extend_from_slice(&command_code.to_be_bytes());
    let mut out = Vec::with_capacity(10 + body.len());
    encode_command_header(
        TpmCommandHeader {
            tag: TPM_ST_SESSIONS,
            size: (10 + body.len()) as u32,
            command_code: TPM_CC_POLICY_COMMAND_CODE,
        },
        &mut out,
    );
    out.extend_from_slice(&body);
    out
}

/// Build a TPM2_Startup command.
pub fn build_startup_command(startup_type: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(12);
    encode_command_header(
        TpmCommandHeader {
            tag: TPM_ST_NO_SESSIONS,
            size: 12,
            command_code: TPM_CC_STARTUP,
        },
        &mut out,
    );
    out.extend_from_slice(&startup_type.to_be_bytes());
    out
}
