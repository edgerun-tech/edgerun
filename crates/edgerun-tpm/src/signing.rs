use std::path::Path;

use edgerun_core::crypto::signature_input;

use crate::constants::TPM_CC_SIGN;
use crate::device::TpmDevice;
use crate::traits::{TpmSigningKey, TpmTransport};
use crate::types::*;
use crate::wire::encode_parsed_signature;

// ---------------------------------------------------------------------------
// LinuxTpmSigningKey
// ---------------------------------------------------------------------------

/// Concrete `TpmSigningKey` backed by a Linux TPM device (`/dev/tpmrm0`).
#[derive(Clone, Debug)]
pub struct LinuxTpmSigningKey {
    device_path: std::path::PathBuf,
    handle: TpmHandle,
    authorization_mode: TpmAuthorizationMode,
}

impl LinuxTpmSigningKey {
    pub fn new(device_path: impl Into<std::path::PathBuf>, handle: TpmHandle) -> Self {
        Self {
            device_path: device_path.into(),
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

    pub fn device_path(&self) -> &Path {
        &self.device_path
    }

    pub fn handle(&self) -> TpmHandle {
        self.handle
    }

    pub fn authorization_mode(&self) -> &TpmAuthorizationMode {
        &self.authorization_mode
    }
}

impl TpmSigningKey for LinuxTpmSigningKey {
    fn key_info(&self) -> Result<TpmKeyInfo, TpmError> {
        use crate::device::LinuxTpmDevice;
        let mut device = TpmDevice::new(LinuxTpmDevice::new(self.device_path.clone()));
        device.read_key_info(self.handle)
    }

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, TpmError> {
        use crate::device::LinuxTpmDevice;
        let key_info = self.key_info()?;
        let params = sign_params_for_message(self.handle, &key_info.algorithm, message)?;
        let mut device = TpmDevice::new(LinuxTpmDevice::new(self.device_path.clone()));
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
    signature_input(sig_domain_tag, record_hash)
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
        | TpmSignatureAlgorithm::EcSchnorr => edgerun_core::crypto::sha256(message).to_vec(),
        TpmSignatureAlgorithm::EcdsaP384Sha384 => edgerun_core::crypto::sha384(message),
        TpmSignatureAlgorithm::Eddsa => edgerun_core::crypto::sha512(message),
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
