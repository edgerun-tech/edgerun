use crate::prelude::v1::*;
use alloc::vec::Vec;

use crate::constants::TPM_CC_SIGN;
use crate::device::TpmDevice;
use crate::traits::{TpmSigningKey, TpmTransport};
use crate::types::*;
use crate::wire::encode_parsed_signature;

// ---------------------------------------------------------------------------
// TpmTransportSigningKey
// ---------------------------------------------------------------------------

/// `TpmSigningKey` backed by an arbitrary byte-level TPM transport.
#[derive(Clone, Debug)]
pub struct TpmTransportSigningKey<T> {
    transport: T,
    handle: TpmHandle,
    authorization_mode: TpmAuthorizationMode,
}

impl<T> TpmTransportSigningKey<T> {
    pub fn new(transport: T, handle: TpmHandle) -> Self {
        Self {
            transport,
            handle,
            authorization_mode: TpmAuthorizationMode::None,
        }
    }

    pub fn with_auth_value(mut self, auth_value: impl Into<Vec<u8>>) -> Self {
        self.authorization_mode = TpmAuthorizationMode::Password(TpmPasswordAuthSession {
            auth_value: auth_value.into(),
            session_attributes: 0,
        });
        self
    }

    pub fn with_password_auth_session(mut self, auth: TpmPasswordAuthSession) -> Self {
        self.authorization_mode = TpmAuthorizationMode::Password(auth);
        self
    }

    pub fn with_policy_session_runner(mut self, runner: TpmPolicySessionRunner) -> Self {
        self.authorization_mode = TpmAuthorizationMode::Policy(runner);
        self
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn handle(&self) -> TpmHandle {
        self.handle
    }

    pub fn authorization_mode(&self) -> &TpmAuthorizationMode {
        &self.authorization_mode
    }
}

impl<T> TpmSigningKey for TpmTransportSigningKey<T>
where
    T: TpmTransport + Clone,
{
    fn key_info(&self) -> Result<TpmKeyInfo, TpmError> {
        let mut device = TpmDevice::new(self.transport.clone());
        device.read_key_info(self.handle)
    }

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, TpmError> {
        let key_info = self.key_info()?;
        let params = sign_params_for_message(self.handle, &key_info.algorithm, message)?;
        let mut device = TpmDevice::new(self.transport.clone());
        let parsed = sign_prehashed_with_device(&mut device, &self.authorization_mode, &params)?;
        Ok(encode_parsed_signature(&parsed))
    }
}

// ---------------------------------------------------------------------------
// TpmPolicySessionRunner
// ---------------------------------------------------------------------------

impl TpmPolicySessionRunner {
    pub fn with_pcr_policy(mut self, selection: TpmPcrSelection, digest: Vec<u8>) -> Self {
        self.pcr_policy = Some(selection);
        self.pcr_digest = digest;
        self
    }

    pub fn with_policy_authorize(
        mut self,
        approved_policy: Vec<u8>,
        policy_ref: Vec<u8>,
        key_sign_name: Vec<u8>,
        check_ticket_hierarchy: u32,
        check_ticket_digest: Vec<u8>,
    ) -> Self {
        self.authorize_policy = Some(TpmPolicyAuthorizeParams {
            session: TpmPolicySession {
                session_handle: 0,
                session_attributes: 0,
            },
            approved_policy,
            policy_ref,
            key_sign_name,
            check_ticket_hierarchy,
            check_ticket_digest,
        });
        self
    }

    pub fn start_policy_session<T: TpmTransport>(
        &self,
        device: &mut TpmDevice<T>,
    ) -> Result<TpmPolicySession, TpmError> {
        let response = device.start_auth_session(&TpmStartAuthSessionParams {
            tpm_key: TpmHandle(crate::constants::TPM_RH_NULL),
            bind: TpmHandle(crate::constants::TPM_RH_NULL),
            nonce_caller: self.nonce_caller.clone(),
            session_type: TpmSessionType::Policy,
            symmetric: TpmSymmetricDefinition::Null,
            auth_hash: self.auth_hash,
        })?;
        Ok(TpmPolicySession {
            session_handle: response.session_handle,
            session_attributes: self.session_attributes,
        })
    }

    pub fn sign_authorized<T: TpmTransport>(
        &self,
        device: &mut TpmDevice<T>,
        params: &TpmSignCommandParams,
    ) -> Result<TpmParsedSignature, TpmError> {
        let session = self.start_policy_session(device)?;
        if let Some(selection) = &self.pcr_policy {
            device.policy_pcr(&TpmPolicyPcrParams {
                session,
                pcr_digest: self.pcr_digest.clone(),
                selection: selection.clone(),
            })?;
        }
        if let Some(authorize) = &self.authorize_policy {
            let mut params = authorize.clone();
            params.session = session;
            device.policy_authorize(&params)?;
        }
        device.policy_command_code(session, TPM_CC_SIGN)?;
        device.sign_raw_with_policy_session(params, session)
    }
}

// ---------------------------------------------------------------------------
// High-level signing helpers
// ---------------------------------------------------------------------------

/// Build the signature input for a mesh record.
pub fn signature_input_for_record(sig_domain_tag: &str, record_hash: &[u8]) -> Vec<u8> {
    let mut input = Vec::with_capacity(sig_domain_tag.len() + 1 + record_hash.len());
    input.extend_from_slice(sig_domain_tag.as_bytes());
    input.push(0);
    input.extend_from_slice(record_hash);
    input
}

/// Sign a mesh record hash with a TPM key.
pub fn sign_record_with_tpm(
    key: &dyn TpmSigningKey,
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, TpmError> {
    key.sign_message(&signature_input_for_record(sig_domain_tag, record_hash))
}

/// Sign a mesh record hash with algorithm validation.
pub fn sign_record_with_tpm_checked(
    key: &dyn TpmSigningKey,
    expected_algorithms: &[TpmSignatureAlgorithm],
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, TpmError> {
    let key_info = key.key_info()?;
    if !expected_algorithms.is_empty()
        && !expected_algorithms
            .iter()
            .any(|algorithm| algorithm == &key_info.algorithm)
    {
        return Err(TpmError::UnsupportedAlgorithm(key_info.algorithm));
    }
    key.sign_message(&signature_input_for_record(sig_domain_tag, record_hash))
}

/// Build signing parameters from a raw message.
pub fn sign_params_for_message(
    key_handle: TpmHandle,
    algorithm: &TpmSignatureAlgorithm,
    message: &[u8],
) -> Result<TpmSignCommandParams, TpmError> {
    Ok(TpmSignCommandParams {
        key_handle,
        digest: hash_message_for_algorithm(algorithm, message)?,
        scheme: default_sign_scheme_for_algorithm(algorithm),
        validation_ticket_tag: crate::constants::TPM_ST_HASHCHECK,
        validation_ticket_hierarchy: crate::constants::TPM_RH_NULL,
        validation_digest: Vec::new(),
    })
}

/// Dispatch a sign operation based on the authorization mode.
pub fn sign_prehashed_with_device<T: TpmTransport>(
    device: &mut TpmDevice<T>,
    authorization_mode: &TpmAuthorizationMode,
    params: &TpmSignCommandParams,
) -> Result<TpmParsedSignature, TpmError> {
    match authorization_mode {
        TpmAuthorizationMode::None => {
            let empty_auth = TpmPasswordAuthSession {
                auth_value: Vec::new(),
                session_attributes: 0,
            };
            device.sign_raw_with_password_auth(params, &empty_auth)
        }
        TpmAuthorizationMode::Password(auth) => device.sign_raw_with_password_auth(params, auth),
        TpmAuthorizationMode::Policy(runner) => runner.sign_authorized(device, params),
    }
}

/// Hash a message for a given signature algorithm.
pub fn hash_message_for_algorithm(
    algorithm: &TpmSignatureAlgorithm,
    message: &[u8],
) -> Result<Vec<u8>, TpmError> {
    Ok(match algorithm {
        TpmSignatureAlgorithm::RsaPkcs1v15Sha256
        | TpmSignatureAlgorithm::RsaPssSha256
        | TpmSignatureAlgorithm::EcdsaP256Sha256
        | TpmSignatureAlgorithm::EcSchnorr => edgerun_crypto::sha256(message).to_vec(),
        TpmSignatureAlgorithm::EcdsaP384Sha384 => edgerun_crypto::sha384(message).to_vec(),
        TpmSignatureAlgorithm::Eddsa => edgerun_crypto::sha512(message).to_vec(),
        TpmSignatureAlgorithm::Opaque(v) => {
            return Err(TpmError::UnsupportedAlgorithm(
                TpmSignatureAlgorithm::Opaque(v.clone()),
            ))
        }
    })
}

/// Get the default TPM signing scheme for an algorithm.
pub fn default_sign_scheme_for_algorithm(algorithm: &TpmSignatureAlgorithm) -> TpmSignatureScheme {
    match algorithm {
        TpmSignatureAlgorithm::RsaPkcs1v15Sha256 => TpmSignatureScheme {
            scheme: 0x0014,
            hash_algorithm: Some(TpmNameAlgorithm::Sha256),
        },
        TpmSignatureAlgorithm::RsaPssSha256 => TpmSignatureScheme {
            scheme: 0x0016,
            hash_algorithm: Some(TpmNameAlgorithm::Sha256),
        },
        TpmSignatureAlgorithm::EcdsaP256Sha256 => TpmSignatureScheme {
            scheme: 0x0018,
            hash_algorithm: Some(TpmNameAlgorithm::Sha256),
        },
        TpmSignatureAlgorithm::EcdsaP384Sha384 => TpmSignatureScheme {
            scheme: 0x0018,
            hash_algorithm: Some(TpmNameAlgorithm::Sha384),
        },
        TpmSignatureAlgorithm::EcSchnorr => TpmSignatureScheme {
            scheme: 0x001C,
            hash_algorithm: Some(TpmNameAlgorithm::Sha256),
        },
        TpmSignatureAlgorithm::Eddsa => TpmSignatureScheme {
            scheme: 0x001A,
            hash_algorithm: Some(TpmNameAlgorithm::Sha512),
        },
        TpmSignatureAlgorithm::Opaque(_) => TpmSignatureScheme {
            scheme: 0x0010,
            hash_algorithm: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{
        TPM_ALG_ECC, TPM_ALG_ECDSA, TPM_ALG_NULL, TPM_ALG_SHA256, TPM_CC_READ_PUBLIC, TPM_CC_SIGN,
        TPM_ECC_NIST_P256, TPM_RC_SUCCESS, TPM_ST_NO_SESSIONS, TPM_ST_SESSIONS,
    };
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct FakeTransport {
        commands: Arc<Mutex<Vec<u32>>>,
    }

    impl FakeTransport {
        fn commands(&self) -> Vec<u32> {
            self.commands.lock().unwrap().clone()
        }
    }

    impl TpmTransport for FakeTransport {
        fn transact(&mut self, command: &[u8]) -> Result<Vec<u8>, TpmError> {
            if command.len() < 10 {
                return Err(TpmError::Protocol("command too short".into()));
            }

            let command_code = u32::from_be_bytes([command[6], command[7], command[8], command[9]]);
            self.commands.lock().unwrap().push(command_code);
            match command_code {
                TPM_CC_READ_PUBLIC => Ok(fake_read_public_response()),
                TPM_CC_SIGN => Ok(fake_sign_response()),
                other => Err(TpmError::Protocol(format!(
                    "unexpected command 0x{other:08x}"
                ))),
            }
        }
    }

    fn response(tag: u16, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(10 + body.len());
        out.extend_from_slice(&tag.to_be_bytes());
        out.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
        out.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        out.extend_from_slice(body);
        out
    }

    fn fake_read_public_response() -> Vec<u8> {
        let mut public = Vec::new();
        public.extend_from_slice(&TPM_ALG_ECC.to_be_bytes());
        public.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        public.extend_from_slice(&0x0004_0000u32.to_be_bytes());
        public.extend_from_slice(&0u16.to_be_bytes());
        public.extend_from_slice(&TPM_ALG_NULL.to_be_bytes());
        public.extend_from_slice(&TPM_ALG_ECDSA.to_be_bytes());
        public.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        public.extend_from_slice(&TPM_ECC_NIST_P256.to_be_bytes());
        public.extend_from_slice(&TPM_ALG_NULL.to_be_bytes());
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&[0x11; 32]);
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&[0x22; 32]);

        let mut name = Vec::new();
        name.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        name.extend_from_slice(&[0xAA; 32]);

        let mut body = Vec::new();
        body.extend_from_slice(&(public.len() as u16).to_be_bytes());
        body.extend_from_slice(&public);
        body.extend_from_slice(&(name.len() as u16).to_be_bytes());
        body.extend_from_slice(&name);
        body.extend_from_slice(&(name.len() as u16).to_be_bytes());
        body.extend_from_slice(&name);
        response(TPM_ST_NO_SESSIONS, &body)
    }

    fn fake_sign_response() -> Vec<u8> {
        let mut signature = Vec::new();
        signature.extend_from_slice(&TPM_ALG_ECDSA.to_be_bytes());
        signature.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        signature.extend_from_slice(&32u16.to_be_bytes());
        signature.extend_from_slice(&[0x33; 32]);
        signature.extend_from_slice(&32u16.to_be_bytes());
        signature.extend_from_slice(&[0x44; 32]);

        let mut body = Vec::new();
        body.extend_from_slice(&(signature.len() as u32).to_be_bytes());
        body.extend_from_slice(&signature);
        response(TPM_ST_SESSIONS, &body)
    }

    #[test]
    fn transport_signing_key_uses_generic_transport() {
        let transport = FakeTransport::default();
        let key = TpmTransportSigningKey::new(transport.clone(), TpmHandle(0x8100_0001));

        let info = key.key_info().unwrap();
        assert_eq!(info.algorithm, TpmSignatureAlgorithm::EcdsaP256Sha256);
        assert_eq!(info.public_key.len(), 64);

        let signature = key.sign_message(b"hello").unwrap();
        assert_eq!(signature.len(), 72);

        assert_eq!(
            transport.commands(),
            vec![TPM_CC_READ_PUBLIC, TPM_CC_READ_PUBLIC, TPM_CC_SIGN]
        );
    }
}
