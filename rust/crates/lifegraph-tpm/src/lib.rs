use lifegraph_core::crypto::signature_input;
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const TPM_ST_NO_SESSIONS: u16 = 0x8001;
pub const TPM_ST_SESSIONS: u16 = 0x8002;
pub const TPM_RC_SUCCESS: u32 = 0;
pub const TPM_CC_READ_PUBLIC: u32 = 0x0000_0173;
pub const TPM_CC_SIGN: u32 = 0x0000_015D;
pub const TPM_CC_HASH: u32 = 0x0000_017D;
pub const TPM_CC_VERIFY_SIGNATURE: u32 = 0x0000_0177;
pub const TPM_CC_START_AUTH_SESSION: u32 = 0x0000_0176;
pub const TPM_CC_POLICY_COMMAND_CODE: u32 = 0x0000_016C;
pub const TPM_CC_POLICY_PCR: u32 = 0x0000_017F;
pub const TPM_CC_POLICY_AUTHORIZE: u32 = 0x0000_016A;
pub const TPM_RS_PW: u32 = 0x4000_0009;
pub const TPM_RH_OWNER: u32 = 0x4000_0001;
pub const TPM_RH_NULL: u32 = 0x4000_0007;
pub const TPM_ST_HASHCHECK: u16 = 0x8024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TpmSignatureAlgorithm {
    RsaPkcs1v15Sha256,
    RsaPssSha256,
    EcdsaP256Sha256,
    EcdsaP384Sha384,
    EcSchnorr,
    Eddsa,
    Opaque(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TpmAssuranceLevel {
    SoftwareSimulated,
    DiscreteTpm,
    IntegratedTpm,
    Certified(String),
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpmHandle(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmKeyInfo {
    pub key_name: String,
    pub algorithm: TpmSignatureAlgorithm,
    pub public_key: Vec<u8>,
    pub attestation_blob: Option<Vec<u8>>,
    pub assurance_level: TpmAssuranceLevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpmCommandHeader {
    pub tag: u16,
    pub size: u32,
    pub command_code: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpmResponseHeader {
    pub tag: u16,
    pub size: u32,
    pub response_code: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpmPublicObjectType {
    Rsa,
    Ecc,
    KeyedHash,
    SymCipher,
    Unknown(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpmNameAlgorithm {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
    Null,
    Unknown(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpmEccCurve {
    NistP256,
    NistP384,
    Curve25519,
    Unknown(u16),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmPublicAreaInfo {
    pub object_type: TpmPublicObjectType,
    pub name_algorithm: TpmNameAlgorithm,
    pub object_attributes: u32,
    pub auth_policy: Vec<u8>,
    pub parameters: Vec<u8>,
    pub unique: Vec<u8>,
    pub curve: Option<TpmEccCurve>,
    pub key_bits: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmReadPublicInfo {
    pub public_area: TpmPublicAreaInfo,
    pub public_area_raw: Vec<u8>,
    pub name: Vec<u8>,
    pub qualified_name: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmSignatureScheme {
    pub scheme: u16,
    pub hash_algorithm: Option<TpmNameAlgorithm>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmSignCommandParams {
    pub key_handle: TpmHandle,
    pub digest: Vec<u8>,
    pub scheme: TpmSignatureScheme,
    pub validation_ticket_tag: u16,
    pub validation_ticket_hierarchy: u32,
    pub validation_digest: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmHashParams {
    pub data: Vec<u8>,
    pub hash_algorithm: TpmNameAlgorithm,
    pub hierarchy: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmHashCheckTicket {
    pub tag: u16,
    pub hierarchy: u32,
    pub digest: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmHashResponse {
    pub digest: Vec<u8>,
    pub validation: TpmHashCheckTicket,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmPasswordAuthSession {
    pub auth_value: Vec<u8>,
    pub session_attributes: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpmSessionType {
    Hmac = 0x00,
    Policy = 0x01,
    Trial = 0x03,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpmSymmetricDefinition {
    Null,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmStartAuthSessionParams {
    pub tpm_key: TpmHandle,
    pub bind: TpmHandle,
    pub nonce_caller: Vec<u8>,
    pub session_type: TpmSessionType,
    pub symmetric: TpmSymmetricDefinition,
    pub auth_hash: TpmNameAlgorithm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpmPolicySession {
    pub session_handle: u32,
    pub session_attributes: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmPcrSelection {
    pub hash_algorithm: TpmNameAlgorithm,
    pub pcrs: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmPolicyPcrParams {
    pub session: TpmPolicySession,
    pub pcr_digest: Vec<u8>,
    pub selection: TpmPcrSelection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmPolicyAuthorizeParams {
    pub session: TpmPolicySession,
    pub approved_policy: Vec<u8>,
    pub policy_ref: Vec<u8>,
    pub key_sign_name: Vec<u8>,
    pub check_ticket_hierarchy: u32,
    pub check_ticket_digest: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmAuthCommand {
    pub session_handle: u32,
    pub nonce: Vec<u8>,
    pub session_attributes: u8,
    pub hmac: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmStartAuthSessionResponse {
    pub session_handle: u32,
    pub nonce_tpm: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TpmParsedSignature {
    Rsa {
        scheme: u16,
        hash_algorithm: TpmNameAlgorithm,
        signature: Vec<u8>,
    },
    Ecc {
        scheme: u16,
        hash_algorithm: TpmNameAlgorithm,
        r: Vec<u8>,
        s: Vec<u8>,
    },
    Opaque {
        scheme: u16,
        bytes: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TpmError {
    Provider(String),
    UnsupportedAlgorithm(TpmSignatureAlgorithm),
    UnsupportedPublicArea,
    Io(String),
    Protocol(String),
    TpmResponseCode(u32),
}

impl core::fmt::Display for TpmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Provider(msg) => f.write_str(msg),
            Self::UnsupportedAlgorithm(algorithm) => {
                write!(f, "unsupported TPM signature algorithm: {algorithm:?}")
            }
            Self::UnsupportedPublicArea => f.write_str("unsupported TPM public area"),
            Self::Io(msg) => write!(f, "TPM I/O error: {msg}"),
            Self::Protocol(msg) => write!(f, "TPM protocol error: {msg}"),
            Self::TpmResponseCode(code) => write!(f, "TPM returned response code 0x{code:08x}"),
        }
    }
}

impl std::error::Error for TpmError {}

pub trait TpmSigningKey {
    fn key_info(&self) -> Result<TpmKeyInfo, TpmError>;

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, TpmError>;
}

pub trait TpmTransport {
    fn transact(&mut self, command: &[u8]) -> Result<Vec<u8>, TpmError>;
}

#[derive(Clone, Debug)]
pub struct LinuxTpmDevice {
    path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TpmAuthorizationMode {
    None,
    Password(TpmPasswordAuthSession),
    Policy(TpmPolicySessionRunner),
}

#[derive(Clone, Debug)]
pub struct LinuxTpmSigningKey {
    device_path: PathBuf,
    handle: TpmHandle,
    authorization_mode: TpmAuthorizationMode,
}

impl LinuxTpmDevice {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmPolicySessionRunner {
    pub nonce_caller: Vec<u8>,
    pub auth_hash: TpmNameAlgorithm,
    pub session_attributes: u8,
    pub pcr_policy: Option<TpmPcrSelection>,
    pub pcr_digest: Vec<u8>,
    pub authorize_policy: Option<TpmPolicyAuthorizeParams>,
}

impl Default for TpmPolicySessionRunner {
    fn default() -> Self {
        Self {
            nonce_caller: vec![0u8; 16],
            auth_hash: TpmNameAlgorithm::Sha256,
            session_attributes: 0,
            pcr_policy: None,
            pcr_digest: Vec::new(),
            authorize_policy: None,
        }
    }
}

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
            tpm_key: TpmHandle(TPM_RH_NULL),
            bind: TpmHandle(TPM_RH_NULL),
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

impl LinuxTpmSigningKey {
    pub fn new(device_path: impl Into<PathBuf>, handle: TpmHandle) -> Self {
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
        let mut device = TpmDevice::new(LinuxTpmDevice::new(self.device_path.clone()));
        device.read_key_info(self.handle)
    }

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, TpmError> {
        let key_info = self.key_info()?;
        let params = sign_params_for_message(self.handle, &key_info.algorithm, message)?;
        let mut device = TpmDevice::new(LinuxTpmDevice::new(self.device_path.clone()));
        let parsed = sign_prehashed_with_device(&mut device, &self.authorization_mode, &params)?;
        Ok(encode_parsed_signature(&parsed))
    }
}

impl TpmTransport for LinuxTpmDevice {
    fn transact(&mut self, command: &[u8]) -> Result<Vec<u8>, TpmError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|err| TpmError::Io(format!("open {}: {err}", self.path.display())))?;
        file.write_all(command)
            .map_err(|err| TpmError::Io(format!("write {}: {err}", self.path.display())))?;

        let mut header_buf = [0u8; 10];
        file.read_exact(&mut header_buf)
            .map_err(|err| TpmError::Io(format!("read header {}: {err}", self.path.display())))?;
        let header = parse_response_header(&header_buf)?;
        if header.size < header_buf.len() as u32 {
            return Err(TpmError::Protocol(format!(
                "response too small: {}",
                header.size
            )));
        }
        let mut out = Vec::with_capacity(header.size as usize);
        out.extend_from_slice(&header_buf);
        let remaining = header.size as usize - header_buf.len();
        if remaining > 0 {
            let mut rest = vec![0u8; remaining];
            file.read_exact(&mut rest)
                .map_err(|err| TpmError::Io(format!("read body {}: {err}", self.path.display())))?;
            out.extend_from_slice(&rest);
        }
        Ok(out)
    }
}

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
    pub fn transmit_command(&mut self, command: &[u8]) -> Result<Vec<u8>, TpmError> {
        let response = self.transport.transact(command)?;
        ensure_success_response(&response)?;
        Ok(response)
    }

    pub fn read_public(&mut self, handle: TpmHandle) -> Result<TpmReadPublicInfo, TpmError> {
        let response = self.transmit_command(&build_read_public_command(handle))?;
        parse_read_public_response(&response)
    }

    pub fn read_key_info(&mut self, handle: TpmHandle) -> Result<TpmKeyInfo, TpmError> {
        let public = self.read_public(handle)?;
        key_info_from_read_public(handle, &public)
    }

    pub fn hash(&mut self, params: &TpmHashParams) -> Result<TpmHashResponse, TpmError> {
        let response = self.transmit_command(&build_hash_command(params))?;
        parse_hash_response(&response)
    }

    pub fn sign_raw(
        &mut self,
        params: &TpmSignCommandParams,
    ) -> Result<TpmParsedSignature, TpmError> {
        let response = self.transmit_command(&build_sign_command(params))?;
        parse_sign_response(&response)
    }

    pub fn sign_raw_with_password_auth(
        &mut self,
        params: &TpmSignCommandParams,
        auth: &TpmPasswordAuthSession,
    ) -> Result<TpmParsedSignature, TpmError> {
        let response =
            self.transmit_command(&build_sign_command_with_password_auth(params, auth))?;
        parse_sign_response(&response)
    }

    pub fn start_auth_session(
        &mut self,
        params: &TpmStartAuthSessionParams,
    ) -> Result<TpmStartAuthSessionResponse, TpmError> {
        let response = self.transmit_command(&build_start_auth_session_command(params))?;
        parse_start_auth_session_response(&response)
    }

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

    pub fn policy_pcr(&mut self, params: &TpmPolicyPcrParams) -> Result<(), TpmError> {
        let response = self.transmit_command(&build_policy_pcr_command(params))?;
        ensure_success_response(&response)?;
        Ok(())
    }

    pub fn policy_authorize(&mut self, params: &TpmPolicyAuthorizeParams) -> Result<(), TpmError> {
        let response = self.transmit_command(&build_policy_authorize_command(params))?;
        ensure_success_response(&response)?;
        Ok(())
    }

    pub fn sign_raw_with_policy_session(
        &mut self,
        params: &TpmSignCommandParams,
        session: TpmPolicySession,
    ) -> Result<TpmParsedSignature, TpmError> {
        let response =
            self.transmit_command(&build_sign_command_with_policy_session(params, session))?;
        parse_sign_response(&response)
    }

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
}

pub fn signature_input_for_record(sig_domain_tag: &str, record_hash: &[u8]) -> Vec<u8> {
    signature_input(sig_domain_tag, record_hash)
}

pub fn sign_record_with_tpm(
    key: &dyn TpmSigningKey,
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, TpmError> {
    key.sign_message(&signature_input_for_record(sig_domain_tag, record_hash))
}

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

pub fn sign_params_for_message(
    key_handle: TpmHandle,
    algorithm: &TpmSignatureAlgorithm,
    message: &[u8],
) -> Result<TpmSignCommandParams, TpmError> {
    Ok(TpmSignCommandParams {
        key_handle,
        digest: hash_message_for_algorithm(algorithm, message)?,
        scheme: default_sign_scheme_for_algorithm(algorithm),
        validation_ticket_tag: TPM_ST_HASHCHECK,
        validation_ticket_hierarchy: TPM_RH_NULL,
        validation_digest: Vec::new(),
    })
}

pub fn sign_prehashed_with_device<T: TpmTransport>(
    device: &mut TpmDevice<T>,
    authorization_mode: &TpmAuthorizationMode,
    params: &TpmSignCommandParams,
) -> Result<TpmParsedSignature, TpmError> {
    match authorization_mode {
        TpmAuthorizationMode::None => device.sign_raw(params),
        TpmAuthorizationMode::Password(auth) => device.sign_raw_with_password_auth(params, auth),
        TpmAuthorizationMode::Policy(runner) => runner.sign_authorized(device, params),
    }
}

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

pub fn build_sign_command(params: &TpmSignCommandParams) -> Vec<u8> {
    build_sign_command_body(params, None)
}

pub fn build_sign_command_with_password_auth(
    params: &TpmSignCommandParams,
    auth: &TpmPasswordAuthSession,
) -> Vec<u8> {
    build_sign_command_body(params, Some(auth))
}

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

pub fn build_verify_signature_command(
    key_handle: TpmHandle,
    digest: &[u8],
    signature: &TpmParsedSignature,
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&key_handle.0.to_be_bytes());
    body.extend_from_slice(&(digest.len() as u16).to_be_bytes());
    body.extend_from_slice(digest);
    body.extend_from_slice(&encode_parsed_signature(signature));

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

fn build_sign_command_body(
    params: &TpmSignCommandParams,
    auth: Option<&TpmPasswordAuthSession>,
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
            let auth_area = build_password_auth_area(auth);
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
            out.extend_from_slice(&((auth_area.len()) as u32).to_be_bytes());
            out.extend_from_slice(&auth_area);
            out.extend_from_slice(&parameters[4..]);
            out
        }
    }
}

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

pub fn build_password_auth_area(auth: &TpmPasswordAuthSession) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 2 + 1 + 2 + auth.auth_value.len());
    out.extend_from_slice(&TPM_RS_PW.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.push(auth.session_attributes);
    out.extend_from_slice(&(auth.auth_value.len() as u16).to_be_bytes());
    out.extend_from_slice(&auth.auth_value);
    out
}

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

fn encode_symmetric_definition(value: TpmSymmetricDefinition) -> [u8; 2] {
    match value {
        TpmSymmetricDefinition::Null => 0x0010u16.to_be_bytes(),
    }
}

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

pub fn hash_message_for_algorithm(
    algorithm: &TpmSignatureAlgorithm,
    message: &[u8],
) -> Result<Vec<u8>, TpmError> {
    Ok(match algorithm {
        TpmSignatureAlgorithm::RsaPkcs1v15Sha256
        | TpmSignatureAlgorithm::RsaPssSha256
        | TpmSignatureAlgorithm::EcdsaP256Sha256
        | TpmSignatureAlgorithm::EcSchnorr => Sha256::digest(message).to_vec(),
        TpmSignatureAlgorithm::EcdsaP384Sha384 => Sha384::digest(message).to_vec(),
        TpmSignatureAlgorithm::Eddsa => Sha512::digest(message).to_vec(),
        TpmSignatureAlgorithm::Opaque(v) => {
            return Err(TpmError::UnsupportedAlgorithm(
                TpmSignatureAlgorithm::Opaque(v.clone()),
            ))
        }
    })
}

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

pub fn encode_command_header(header: TpmCommandHeader, out: &mut Vec<u8>) {
    out.extend_from_slice(&header.tag.to_be_bytes());
    out.extend_from_slice(&header.size.to_be_bytes());
    out.extend_from_slice(&header.command_code.to_be_bytes());
}

pub fn parse_response_header(bytes: &[u8]) -> Result<TpmResponseHeader, TpmError> {
    if bytes.len() < 10 {
        return Err(TpmError::Protocol(
            "response header shorter than 10 bytes".into(),
        ));
    }
    Ok(TpmResponseHeader {
        tag: u16::from_be_bytes([bytes[0], bytes[1]]),
        size: u32::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
        response_code: u32::from_be_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]),
    })
}

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

pub fn parse_sign_response(response: &[u8]) -> Result<TpmParsedSignature, TpmError> {
    let header = ensure_success_response(response)?;
    let mut cursor = 10usize;
    let parameter_bytes = if header.tag == TPM_ST_SESSIONS && response.len() >= 14 {
        let parameter_size =
            u32::from_be_bytes([response[10], response[11], response[12], response[13]]) as usize;
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

fn parse_public_area(bytes: &[u8]) -> Result<TpmPublicAreaInfo, TpmError> {
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

fn read_tpm2b(bytes: &[u8], cursor: &mut usize, label: &str) -> Result<Vec<u8>, TpmError> {
    let len = read_u16(bytes, cursor, label)? as usize;
    if bytes.len().saturating_sub(*cursor) < len {
        return Err(TpmError::Protocol(format!("{label} length exceeds buffer")));
    }
    let out = bytes[*cursor..*cursor + len].to_vec();
    *cursor += len;
    Ok(out)
}

fn read_u16(bytes: &[u8], cursor: &mut usize, label: &str) -> Result<u16, TpmError> {
    if bytes.len().saturating_sub(*cursor) < 2 {
        return Err(TpmError::Protocol(format!("missing u16 field {label}")));
    }
    let out = u16::from_be_bytes([bytes[*cursor], bytes[*cursor + 1]]);
    *cursor += 2;
    Ok(out)
}

fn read_u32(bytes: &[u8], cursor: &mut usize, label: &str) -> Result<u32, TpmError> {
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

fn map_public_object_type(value: u16) -> TpmPublicObjectType {
    match value {
        0x0001 => TpmPublicObjectType::Rsa,
        0x0008 => TpmPublicObjectType::KeyedHash,
        0x0023 => TpmPublicObjectType::Ecc,
        0x0025 => TpmPublicObjectType::SymCipher,
        other => TpmPublicObjectType::Unknown(other),
    }
}

fn encode_name_algorithm(value: TpmNameAlgorithm) -> u16 {
    match value {
        TpmNameAlgorithm::Sha1 => 0x0004,
        TpmNameAlgorithm::Sha256 => 0x000B,
        TpmNameAlgorithm::Sha384 => 0x000C,
        TpmNameAlgorithm::Sha512 => 0x000D,
        TpmNameAlgorithm::Null => 0x0010,
        TpmNameAlgorithm::Unknown(v) => v,
    }
}

fn map_name_algorithm(value: u16) -> TpmNameAlgorithm {
    match value {
        0x0004 => TpmNameAlgorithm::Sha1,
        0x000B => TpmNameAlgorithm::Sha256,
        0x000C => TpmNameAlgorithm::Sha384,
        0x000D => TpmNameAlgorithm::Sha512,
        0x0010 => TpmNameAlgorithm::Null,
        other => TpmNameAlgorithm::Unknown(other),
    }
}

fn map_ecc_curve(value: u16) -> TpmEccCurve {
    match value {
        0x0003 => TpmEccCurve::NistP256,
        0x0004 => TpmEccCurve::NistP384,
        0x0040 => TpmEccCurve::Curve25519,
        other => TpmEccCurve::Unknown(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeTpmKey {
        algorithm: TpmSignatureAlgorithm,
    }

    impl FakeTpmKey {
        fn new(algorithm: TpmSignatureAlgorithm) -> Self {
            Self { algorithm }
        }
    }

    impl TpmSigningKey for FakeTpmKey {
        fn key_info(&self) -> Result<TpmKeyInfo, TpmError> {
            Ok(TpmKeyInfo {
                key_name: "fake-tpm-key".into(),
                algorithm: self.algorithm.clone(),
                public_key: vec![1, 2, 3, 4],
                attestation_blob: Some(vec![9, 9, 9]),
                assurance_level: TpmAssuranceLevel::DiscreteTpm,
            })
        }

        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, TpmError> {
            let mut sig = self.key_info()?.public_key;
            sig.extend_from_slice(message);
            Ok(sig)
        }
    }

    #[test]
    fn signs_protocol_record_input_via_generic_tpm_trait() {
        let key = FakeTpmKey::new(TpmSignatureAlgorithm::EcdsaP256Sha256);
        let sig = sign_record_with_tpm(&key, "lifegraph:v0:sig:test", &[9u8; 32]).unwrap();
        let expected_input = signature_input_for_record("lifegraph:v0:sig:test", &[9u8; 32]);
        assert!(sig.ends_with(&expected_input));
    }

    #[test]
    fn checked_sign_rejects_unexpected_algorithm() {
        let key = FakeTpmKey::new(TpmSignatureAlgorithm::RsaPssSha256);
        let err = sign_record_with_tpm_checked(
            &key,
            &[
                TpmSignatureAlgorithm::EcdsaP256Sha256,
                TpmSignatureAlgorithm::Eddsa,
            ],
            "lifegraph:v0:sig:test",
            &[5u8; 32],
        )
        .unwrap_err();
        assert_eq!(
            err,
            TpmError::UnsupportedAlgorithm(TpmSignatureAlgorithm::RsaPssSha256)
        );
    }

    #[test]
    fn checked_sign_accepts_expected_algorithm() {
        let key = FakeTpmKey::new(TpmSignatureAlgorithm::Eddsa);
        let sig = sign_record_with_tpm_checked(
            &key,
            &[
                TpmSignatureAlgorithm::EcdsaP256Sha256,
                TpmSignatureAlgorithm::Eddsa,
            ],
            "lifegraph:v0:sig:test",
            &[7u8; 32],
        )
        .unwrap();
        assert!(!sig.is_empty());
    }

    #[test]
    fn build_read_public_command_has_expected_wire_shape() {
        let command = build_read_public_command(TpmHandle(0x8100_0001));
        assert_eq!(command.len(), 14);
        assert_eq!(&command[0..2], &TPM_ST_NO_SESSIONS.to_be_bytes());
        assert_eq!(&command[2..6], &(14u32).to_be_bytes());
        assert_eq!(&command[6..10], &TPM_CC_READ_PUBLIC.to_be_bytes());
        assert_eq!(&command[10..14], &0x8100_0001u32.to_be_bytes());
    }

    #[test]
    fn parse_response_header_roundtrips() {
        let bytes = [0x80, 0x01, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x00];
        let header = parse_response_header(&bytes).unwrap();
        assert_eq!(header.tag, TPM_ST_NO_SESSIONS);
        assert_eq!(header.size, 10);
        assert_eq!(header.response_code, TPM_RC_SUCCESS);
    }

    #[test]
    fn parse_read_public_response_extracts_ecc_p256_metadata() {
        let mut public = Vec::new();
        public.extend_from_slice(&0x0023u16.to_be_bytes());
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        public.extend_from_slice(&0x00030072u32.to_be_bytes());
        public.extend_from_slice(&0u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        public.extend_from_slice(&0x0018u16.to_be_bytes());
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        public.extend_from_slice(&0x0003u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&[0xAA; 32]);
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&[0xBB; 32]);

        let mut response = Vec::new();
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        let total_size = 10 + 2 + public.len() + 2 + 4 + 2 + 4;
        response.extend_from_slice(&(total_size as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&(public.len() as u16).to_be_bytes());
        response.extend_from_slice(&public);
        response.extend_from_slice(&4u16.to_be_bytes());
        response.extend_from_slice(&[1, 2, 3, 4]);
        response.extend_from_slice(&4u16.to_be_bytes());
        response.extend_from_slice(&[5, 6, 7, 8]);

        let parsed = parse_read_public_response(&response).unwrap();
        assert_eq!(parsed.public_area.object_type, TpmPublicObjectType::Ecc);
        assert_eq!(parsed.public_area.name_algorithm, TpmNameAlgorithm::Sha256);
        assert_eq!(parsed.public_area.curve, Some(TpmEccCurve::NistP256));
        assert_eq!(parsed.name, vec![1, 2, 3, 4]);
        assert_eq!(parsed.qualified_name, vec![5, 6, 7, 8]);
        assert_eq!(
            infer_signature_algorithm(&parsed.public_area),
            Some(TpmSignatureAlgorithm::EcdsaP256Sha256)
        );
    }

    #[test]
    fn key_info_from_read_public_uses_unique_bytes() {
        let info = TpmReadPublicInfo {
            public_area: TpmPublicAreaInfo {
                object_type: TpmPublicObjectType::Ecc,
                name_algorithm: TpmNameAlgorithm::Sha256,
                object_attributes: 0,
                auth_policy: vec![],
                parameters: vec![],
                unique: vec![0x11, 0x22, 0x33],
                curve: Some(TpmEccCurve::NistP256),
                key_bits: None,
            },
            public_area_raw: vec![],
            name: vec![],
            qualified_name: vec![],
        };
        let key = key_info_from_read_public(TpmHandle(0x8100_0001), &info).unwrap();
        assert_eq!(key.key_name, "tpm:0x81000001");
        assert_eq!(key.algorithm, TpmSignatureAlgorithm::EcdsaP256Sha256);
        assert_eq!(key.public_key, vec![0x11, 0x22, 0x33]);
    }

    #[test]
    fn build_sign_command_has_expected_wire_shape() {
        let command = build_sign_command(&TpmSignCommandParams {
            key_handle: TpmHandle(0x8100_0001),
            digest: vec![0xAA; 32],
            scheme: default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::EcdsaP256Sha256),
            validation_ticket_tag: TPM_ST_HASHCHECK,
            validation_ticket_hierarchy: TPM_RH_NULL,
            validation_digest: vec![],
        });
        assert_eq!(&command[0..2], &TPM_ST_NO_SESSIONS.to_be_bytes());
        assert_eq!(&command[6..10], &TPM_CC_SIGN.to_be_bytes());
        assert_eq!(&command[10..14], &0x8100_0001u32.to_be_bytes());
        assert_eq!(&command[14..16], &32u16.to_be_bytes());
        assert_eq!(
            command.len(),
            u32::from_be_bytes(command[2..6].try_into().unwrap()) as usize
        );
    }

    #[test]
    fn build_hash_command_has_expected_wire_shape() {
        let command = build_hash_command(&TpmHashParams {
            data: b"abc".to_vec(),
            hash_algorithm: TpmNameAlgorithm::Sha1,
            hierarchy: TPM_RH_OWNER,
        });
        assert_eq!(&command[0..2], &TPM_ST_NO_SESSIONS.to_be_bytes());
        assert_eq!(&command[6..10], &TPM_CC_HASH.to_be_bytes());
        assert_eq!(&command[10..12], &3u16.to_be_bytes());
        assert_eq!(&command[12..15], b"abc");
    }

    #[test]
    fn build_verify_signature_command_has_expected_wire_shape() {
        let command = build_verify_signature_command(
            TpmHandle(0x8100_0002),
            &[0xAA; 20],
            &TpmParsedSignature::Rsa {
                scheme: 0x0014,
                hash_algorithm: TpmNameAlgorithm::Sha1,
                signature: vec![0xBB; 16],
            },
        );
        assert_eq!(&command[0..2], &TPM_ST_NO_SESSIONS.to_be_bytes());
        assert_eq!(&command[6..10], &TPM_CC_VERIFY_SIGNATURE.to_be_bytes());
        assert_eq!(&command[10..14], &0x8100_0002u32.to_be_bytes());
    }

    #[test]
    fn build_password_auth_area_has_expected_wire_shape() {
        let auth = TpmPasswordAuthSession {
            auth_value: vec![1, 2, 3],
            session_attributes: 0,
        };
        let area = build_password_auth_area(&auth);
        assert_eq!(&area[0..4], &TPM_RS_PW.to_be_bytes());
        assert_eq!(&area[4..6], &0u16.to_be_bytes());
        assert_eq!(area[6], 0);
        assert_eq!(&area[7..9], &3u16.to_be_bytes());
        assert_eq!(&area[9..12], &[1, 2, 3]);
    }

    #[test]
    fn build_sign_command_with_password_auth_uses_sessions_tag() {
        let command = build_sign_command_with_password_auth(
            &TpmSignCommandParams {
                key_handle: TpmHandle(0x8100_0001),
                digest: vec![0xAA; 32],
                scheme: default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::EcdsaP256Sha256),
                validation_ticket_tag: TPM_ST_HASHCHECK,
                validation_ticket_hierarchy: TPM_RH_NULL,
                validation_digest: vec![],
            },
            &TpmPasswordAuthSession {
                auth_value: vec![0x55, 0x66],
                session_attributes: 0,
            },
        );
        assert_eq!(&command[0..2], &TPM_ST_SESSIONS.to_be_bytes());
        assert_eq!(&command[6..10], &TPM_CC_SIGN.to_be_bytes());
        assert_eq!(&command[10..14], &0x8100_0001u32.to_be_bytes());
        let auth_size = u32::from_be_bytes(command[14..18].try_into().unwrap()) as usize;
        assert_eq!(auth_size, 11);
        assert_eq!(&command[18..22], &TPM_RS_PW.to_be_bytes());
    }

    #[test]
    fn parse_sign_response_extracts_rsa_signature() {
        let mut response = Vec::new();
        let mut body = Vec::new();
        body.extend_from_slice(&0x0016u16.to_be_bytes());
        body.extend_from_slice(&0x000Bu16.to_be_bytes());
        body.extend_from_slice(&4u16.to_be_bytes());
        body.extend_from_slice(&[9, 8, 7, 6]);
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&body);

        let parsed = parse_sign_response(&response).unwrap();
        assert_eq!(
            parsed,
            TpmParsedSignature::Rsa {
                scheme: 0x0016,
                hash_algorithm: TpmNameAlgorithm::Sha256,
                signature: vec![9, 8, 7, 6],
            }
        );
    }

    #[test]
    fn parse_sign_response_extracts_ecc_signature() {
        let mut response = Vec::new();
        let mut body = Vec::new();
        body.extend_from_slice(&0x0018u16.to_be_bytes());
        body.extend_from_slice(&0x000Bu16.to_be_bytes());
        body.extend_from_slice(&2u16.to_be_bytes());
        body.extend_from_slice(&[1, 2]);
        body.extend_from_slice(&2u16.to_be_bytes());
        body.extend_from_slice(&[3, 4]);
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&body);

        let parsed = parse_sign_response(&response).unwrap();
        assert_eq!(
            parsed,
            TpmParsedSignature::Ecc {
                scheme: 0x0018,
                hash_algorithm: TpmNameAlgorithm::Sha256,
                r: vec![1, 2],
                s: vec![3, 4],
            }
        );
    }

    #[test]
    fn encode_parsed_signature_roundtrips_rsa_shape() {
        let parsed = TpmParsedSignature::Rsa {
            scheme: 0x0016,
            hash_algorithm: TpmNameAlgorithm::Sha256,
            signature: vec![9, 8, 7, 6],
        };
        let encoded = encode_parsed_signature(&parsed);
        assert_eq!(&encoded[0..2], &0x0016u16.to_be_bytes());
        assert_eq!(&encoded[2..4], &0x000Bu16.to_be_bytes());
    }

    #[test]
    fn hash_message_for_algorithm_uses_expected_digest_sizes() {
        assert_eq!(
            hash_message_for_algorithm(&TpmSignatureAlgorithm::EcdsaP256Sha256, b"abc")
                .unwrap()
                .len(),
            32
        );
        assert_eq!(
            hash_message_for_algorithm(&TpmSignatureAlgorithm::EcdsaP384Sha384, b"abc")
                .unwrap()
                .len(),
            48
        );
        assert_eq!(
            hash_message_for_algorithm(&TpmSignatureAlgorithm::Eddsa, b"abc")
                .unwrap()
                .len(),
            64
        );
    }

    #[test]
    fn build_start_auth_session_command_has_expected_wire_shape() {
        let command = build_start_auth_session_command(&TpmStartAuthSessionParams {
            tpm_key: TpmHandle(TPM_RH_NULL),
            bind: TpmHandle(TPM_RH_NULL),
            nonce_caller: vec![1, 2, 3, 4],
            session_type: TpmSessionType::Policy,
            symmetric: TpmSymmetricDefinition::Null,
            auth_hash: TpmNameAlgorithm::Sha256,
        });
        assert_eq!(&command[0..2], &TPM_ST_NO_SESSIONS.to_be_bytes());
        assert_eq!(&command[6..10], &TPM_CC_START_AUTH_SESSION.to_be_bytes());
        assert_eq!(&command[10..14], &TPM_RH_NULL.to_be_bytes());
        assert_eq!(&command[14..18], &TPM_RH_NULL.to_be_bytes());
    }

    #[test]
    fn parse_start_auth_session_response_roundtrips() {
        let mut response = Vec::new();
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&(20u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&0x0300_0000u32.to_be_bytes());
        response.extend_from_slice(&4u16.to_be_bytes());
        response.extend_from_slice(&[9, 8, 7, 6]);
        let parsed = parse_start_auth_session_response(&response).unwrap();
        assert_eq!(parsed.session_handle, 0x0300_0000);
        assert_eq!(parsed.nonce_tpm, vec![9, 8, 7, 6]);
    }

    #[test]
    fn parse_hash_response_roundtrips() {
        let mut response = Vec::new();
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&(27u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&4u16.to_be_bytes());
        response.extend_from_slice(&[1, 2, 3, 4]);
        response.extend_from_slice(&TPM_ST_HASHCHECK.to_be_bytes());
        response.extend_from_slice(&TPM_RH_OWNER.to_be_bytes());
        response.extend_from_slice(&3u16.to_be_bytes());
        response.extend_from_slice(&[5, 6, 7]);
        let parsed = parse_hash_response(&response).unwrap();
        assert_eq!(parsed.digest, vec![1, 2, 3, 4]);
        assert_eq!(parsed.validation.tag, TPM_ST_HASHCHECK);
        assert_eq!(parsed.validation.hierarchy, TPM_RH_OWNER);
        assert_eq!(parsed.validation.digest, vec![5, 6, 7]);
    }

    #[test]
    fn build_policy_command_code_command_uses_sessions_tag() {
        let command = build_policy_command_code_command(
            TpmPolicySession {
                session_handle: 0x0300_0000,
                session_attributes: 0,
            },
            TPM_CC_SIGN,
        );
        assert_eq!(&command[0..2], &TPM_ST_SESSIONS.to_be_bytes());
        assert_eq!(&command[6..10], &TPM_CC_POLICY_COMMAND_CODE.to_be_bytes());
        assert_eq!(&command[10..14], &0x0300_0000u32.to_be_bytes());
    }

    #[test]
    fn build_sign_command_with_policy_session_uses_sessions_tag() {
        let command = build_sign_command_with_policy_session(
            &TpmSignCommandParams {
                key_handle: TpmHandle(0x8100_0001),
                digest: vec![0xAA; 32],
                scheme: default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::EcdsaP256Sha256),
                validation_ticket_tag: TPM_ST_HASHCHECK,
                validation_ticket_hierarchy: TPM_RH_NULL,
                validation_digest: vec![],
            },
            TpmPolicySession {
                session_handle: 0x0300_0000,
                session_attributes: 0,
            },
        );
        assert_eq!(&command[0..2], &TPM_ST_SESSIONS.to_be_bytes());
        assert_eq!(&command[6..10], &TPM_CC_SIGN.to_be_bytes());
        assert_eq!(&command[10..14], &0x8100_0001u32.to_be_bytes());
        let auth_size = u32::from_be_bytes(command[14..18].try_into().unwrap()) as usize;
        assert_eq!(auth_size, 9);
        assert_eq!(&command[18..22], &0x0300_0000u32.to_be_bytes());
    }

    #[test]
    fn build_policy_pcr_command_uses_sessions_tag() {
        let command = build_policy_pcr_command(&TpmPolicyPcrParams {
            session: TpmPolicySession {
                session_handle: 0x0300_0000,
                session_attributes: 0,
            },
            pcr_digest: vec![0xAA; 32],
            selection: TpmPcrSelection {
                hash_algorithm: TpmNameAlgorithm::Sha256,
                pcrs: vec![0, 7],
            },
        });
        assert_eq!(&command[0..2], &TPM_ST_SESSIONS.to_be_bytes());
        assert_eq!(&command[6..10], &TPM_CC_POLICY_PCR.to_be_bytes());
        assert_eq!(&command[10..14], &0x0300_0000u32.to_be_bytes());
    }

    struct FakePolicyTransport {
        responses: std::collections::VecDeque<Vec<u8>>,
        commands: Vec<Vec<u8>>,
    }

    impl FakePolicyTransport {
        fn new(responses: Vec<Vec<u8>>) -> Self {
            Self {
                responses: responses.into(),
                commands: Vec::new(),
            }
        }
    }

    impl TpmTransport for FakePolicyTransport {
        fn transact(&mut self, command: &[u8]) -> Result<Vec<u8>, TpmError> {
            self.commands.push(command.to_vec());
            self.responses
                .pop_front()
                .ok_or_else(|| TpmError::Protocol("missing fake response".into()))
        }
    }

    #[test]
    fn policy_session_runner_sequences_start_policy_sign() {
        let start_response = {
            let mut v = Vec::new();
            v.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
            v.extend_from_slice(&(20u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v.extend_from_slice(&0x0300_0000u32.to_be_bytes());
            v.extend_from_slice(&4u16.to_be_bytes());
            v.extend_from_slice(&[1, 2, 3, 4]);
            v
        };
        let policy_response = {
            let mut v = Vec::new();
            v.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
            v.extend_from_slice(&(10u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v
        };
        let sign_response = {
            let mut v = Vec::new();
            let mut body = Vec::new();
            body.extend_from_slice(&0x0018u16.to_be_bytes());
            body.extend_from_slice(&0x000Bu16.to_be_bytes());
            body.extend_from_slice(&2u16.to_be_bytes());
            body.extend_from_slice(&[1, 2]);
            body.extend_from_slice(&2u16.to_be_bytes());
            body.extend_from_slice(&[3, 4]);
            v.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
            v.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v.extend_from_slice(&body);
            v
        };
        let transport =
            FakePolicyTransport::new(vec![start_response, policy_response, sign_response]);
        let mut device = TpmDevice::new(transport);
        let runner = TpmPolicySessionRunner::default();
        let sig = runner
            .sign_authorized(
                &mut device,
                &TpmSignCommandParams {
                    key_handle: TpmHandle(0x8100_0001),
                    digest: vec![0xAA; 32],
                    scheme: default_sign_scheme_for_algorithm(
                        &TpmSignatureAlgorithm::EcdsaP256Sha256,
                    ),
                    validation_ticket_tag: TPM_ST_HASHCHECK,
                    validation_ticket_hierarchy: TPM_RH_NULL,
                    validation_digest: vec![],
                },
            )
            .unwrap();
        assert!(matches!(sig, TpmParsedSignature::Ecc { .. }));
        assert_eq!(device.transport().commands.len(), 3);
        assert_eq!(
            &device.transport().commands[0][6..10],
            &TPM_CC_START_AUTH_SESSION.to_be_bytes()
        );
        assert_eq!(
            &device.transport().commands[1][6..10],
            &TPM_CC_POLICY_COMMAND_CODE.to_be_bytes()
        );
        assert_eq!(
            &device.transport().commands[2][6..10],
            &TPM_CC_SIGN.to_be_bytes()
        );
    }

    #[test]
    fn policy_session_runner_sequences_pcr_then_command_code_then_sign() {
        let start_response = {
            let mut v = Vec::new();
            v.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
            v.extend_from_slice(&(20u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v.extend_from_slice(&0x0300_0000u32.to_be_bytes());
            v.extend_from_slice(&4u16.to_be_bytes());
            v.extend_from_slice(&[1, 2, 3, 4]);
            v
        };
        let policy_pcr_response = {
            let mut v = Vec::new();
            v.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
            v.extend_from_slice(&(10u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v
        };
        let policy_code_response = {
            let mut v = Vec::new();
            v.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
            v.extend_from_slice(&(10u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v
        };
        let sign_response = {
            let mut v = Vec::new();
            let mut body = Vec::new();
            body.extend_from_slice(&0x0018u16.to_be_bytes());
            body.extend_from_slice(&0x000Bu16.to_be_bytes());
            body.extend_from_slice(&2u16.to_be_bytes());
            body.extend_from_slice(&[1, 2]);
            body.extend_from_slice(&2u16.to_be_bytes());
            body.extend_from_slice(&[3, 4]);
            v.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
            v.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v.extend_from_slice(&body);
            v
        };
        let transport = FakePolicyTransport::new(vec![
            start_response,
            policy_pcr_response,
            policy_code_response,
            sign_response,
        ]);
        let mut device = TpmDevice::new(transport);
        let runner = TpmPolicySessionRunner::default().with_pcr_policy(
            TpmPcrSelection {
                hash_algorithm: TpmNameAlgorithm::Sha256,
                pcrs: vec![0, 7],
            },
            vec![0xAA; 32],
        );
        let sig = runner
            .sign_authorized(
                &mut device,
                &TpmSignCommandParams {
                    key_handle: TpmHandle(0x8100_0001),
                    digest: vec![0xAA; 32],
                    scheme: default_sign_scheme_for_algorithm(
                        &TpmSignatureAlgorithm::EcdsaP256Sha256,
                    ),
                    validation_ticket_tag: TPM_ST_HASHCHECK,
                    validation_ticket_hierarchy: TPM_RH_NULL,
                    validation_digest: vec![],
                },
            )
            .unwrap();
        assert!(matches!(sig, TpmParsedSignature::Ecc { .. }));
        assert_eq!(device.transport().commands.len(), 4);
        assert_eq!(
            &device.transport().commands[0][6..10],
            &TPM_CC_START_AUTH_SESSION.to_be_bytes()
        );
        assert_eq!(
            &device.transport().commands[1][6..10],
            &TPM_CC_POLICY_PCR.to_be_bytes()
        );
        assert_eq!(
            &device.transport().commands[2][6..10],
            &TPM_CC_POLICY_COMMAND_CODE.to_be_bytes()
        );
        assert_eq!(
            &device.transport().commands[3][6..10],
            &TPM_CC_SIGN.to_be_bytes()
        );
    }

    #[test]
    fn linux_tpm_signing_key_switches_to_policy_mode() {
        let runner = TpmPolicySessionRunner::default().with_pcr_policy(
            TpmPcrSelection {
                hash_algorithm: TpmNameAlgorithm::Sha256,
                pcrs: vec![0, 7],
            },
            vec![0xAA; 32],
        );
        let key = LinuxTpmSigningKey::new("/dev/tpmrm0", TpmHandle(0x8100_0001))
            .with_policy_session_runner(runner.clone());
        assert_eq!(
            key.authorization_mode(),
            &TpmAuthorizationMode::Policy(runner)
        );
    }

    #[test]
    fn sign_prehashed_with_device_uses_password_mode() {
        let sign_response = {
            let mut v = Vec::new();
            v.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
            v.extend_from_slice(&(20u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v.extend_from_slice(&0x0016u16.to_be_bytes());
            v.extend_from_slice(&0x000Bu16.to_be_bytes());
            v.extend_from_slice(&4u16.to_be_bytes());
            v.extend_from_slice(&[9, 8, 7, 6]);
            v
        };
        let mut device = TpmDevice::new(FakePolicyTransport::new(vec![sign_response]));
        let parsed = sign_prehashed_with_device(
            &mut device,
            &TpmAuthorizationMode::Password(TpmPasswordAuthSession {
                auth_value: vec![0x55],
                session_attributes: 0,
            }),
            &TpmSignCommandParams {
                key_handle: TpmHandle(0x8100_0001),
                digest: vec![0xAA; 32],
                scheme: default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::EcdsaP256Sha256),
                validation_ticket_tag: TPM_ST_HASHCHECK,
                validation_ticket_hierarchy: TPM_RH_NULL,
                validation_digest: vec![],
            },
        )
        .unwrap();
        assert!(matches!(parsed, TpmParsedSignature::Rsa { .. }));
        let commands = &device.transport().commands;
        assert_eq!(commands.len(), 1);
        assert_eq!(&commands[0][0..2], &TPM_ST_SESSIONS.to_be_bytes());
        assert_eq!(&commands[0][6..10], &TPM_CC_SIGN.to_be_bytes());
    }

    #[test]
    fn sign_prehashed_with_device_uses_policy_mode_sequence() {
        let start_response = {
            let mut v = Vec::new();
            v.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
            v.extend_from_slice(&(20u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v.extend_from_slice(&0x0300_0000u32.to_be_bytes());
            v.extend_from_slice(&4u16.to_be_bytes());
            v.extend_from_slice(&[1, 2, 3, 4]);
            v
        };
        let policy_response = {
            let mut v = Vec::new();
            v.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
            v.extend_from_slice(&(10u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v
        };
        let sign_response = {
            let mut v = Vec::new();
            v.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
            v.extend_from_slice(&(20u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v.extend_from_slice(&0x0016u16.to_be_bytes());
            v.extend_from_slice(&0x000Bu16.to_be_bytes());
            v.extend_from_slice(&4u16.to_be_bytes());
            v.extend_from_slice(&[4, 5, 6, 7]);
            v
        };
        let mut device = TpmDevice::new(FakePolicyTransport::new(vec![
            start_response,
            policy_response,
            sign_response,
        ]));
        let parsed = sign_prehashed_with_device(
            &mut device,
            &TpmAuthorizationMode::Policy(TpmPolicySessionRunner::default()),
            &TpmSignCommandParams {
                key_handle: TpmHandle(0x8100_0001),
                digest: vec![0xAA; 32],
                scheme: default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::EcdsaP256Sha256),
                validation_ticket_tag: TPM_ST_HASHCHECK,
                validation_ticket_hierarchy: TPM_RH_NULL,
                validation_digest: vec![],
            },
        )
        .unwrap();
        assert!(matches!(parsed, TpmParsedSignature::Rsa { .. }));
        let commands = &device.transport().commands;
        assert_eq!(commands.len(), 3);
        assert_eq!(
            &commands[0][6..10],
            &TPM_CC_START_AUTH_SESSION.to_be_bytes()
        );
        assert_eq!(
            &commands[1][6..10],
            &TPM_CC_POLICY_COMMAND_CODE.to_be_bytes()
        );
        assert_eq!(&commands[2][6..10], &TPM_CC_SIGN.to_be_bytes());
    }

    #[test]
    fn build_policy_authorize_command_uses_sessions_tag() {
        let command = build_policy_authorize_command(&TpmPolicyAuthorizeParams {
            session: TpmPolicySession {
                session_handle: 0x0300_0000,
                session_attributes: 0,
            },
            approved_policy: vec![0x11, 0x22],
            policy_ref: vec![0x33],
            key_sign_name: vec![0x44, 0x55],
            check_ticket_hierarchy: TPM_RH_NULL,
            check_ticket_digest: vec![0x66, 0x77],
        });
        assert_eq!(&command[0..2], &TPM_ST_SESSIONS.to_be_bytes());
        assert_eq!(&command[6..10], &TPM_CC_POLICY_AUTHORIZE.to_be_bytes());
        assert_eq!(&command[10..14], &0x0300_0000u32.to_be_bytes());
    }

    #[test]
    fn policy_session_runner_sequences_authorize_then_command_code_then_sign() {
        let start_response = {
            let mut v = Vec::new();
            v.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
            v.extend_from_slice(&(20u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v.extend_from_slice(&0x0300_0000u32.to_be_bytes());
            v.extend_from_slice(&4u16.to_be_bytes());
            v.extend_from_slice(&[1, 2, 3, 4]);
            v
        };
        let authorize_response = {
            let mut v = Vec::new();
            v.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
            v.extend_from_slice(&(10u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v
        };
        let policy_code_response = {
            let mut v = Vec::new();
            v.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
            v.extend_from_slice(&(10u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v
        };
        let sign_response = {
            let mut v = Vec::new();
            let mut body = Vec::new();
            body.extend_from_slice(&0x0016u16.to_be_bytes());
            body.extend_from_slice(&0x000Bu16.to_be_bytes());
            body.extend_from_slice(&4u16.to_be_bytes());
            body.extend_from_slice(&[9, 8, 7, 6]);
            v.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
            v.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
            v.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            v.extend_from_slice(&body);
            v
        };
        let transport = FakePolicyTransport::new(vec![
            start_response,
            authorize_response,
            policy_code_response,
            sign_response,
        ]);
        let mut device = TpmDevice::new(transport);
        let runner = TpmPolicySessionRunner::default().with_policy_authorize(
            vec![0x11, 0x22],
            vec![0x33],
            vec![0x44, 0x55],
            TPM_RH_NULL,
            vec![0x66, 0x77],
        );
        let sig = runner
            .sign_authorized(
                &mut device,
                &TpmSignCommandParams {
                    key_handle: TpmHandle(0x8100_0001),
                    digest: vec![0xAA; 32],
                    scheme: default_sign_scheme_for_algorithm(
                        &TpmSignatureAlgorithm::EcdsaP256Sha256,
                    ),
                    validation_ticket_tag: TPM_ST_HASHCHECK,
                    validation_ticket_hierarchy: TPM_RH_NULL,
                    validation_digest: vec![],
                },
            )
            .unwrap();
        assert!(matches!(sig, TpmParsedSignature::Rsa { .. }));
        assert_eq!(device.transport().commands.len(), 4);
        assert_eq!(
            &device.transport().commands[0][6..10],
            &TPM_CC_START_AUTH_SESSION.to_be_bytes()
        );
        assert_eq!(
            &device.transport().commands[1][6..10],
            &TPM_CC_POLICY_AUTHORIZE.to_be_bytes()
        );
        assert_eq!(
            &device.transport().commands[2][6..10],
            &TPM_CC_POLICY_COMMAND_CODE.to_be_bytes()
        );
        assert_eq!(
            &device.transport().commands[3][6..10],
            &TPM_CC_SIGN.to_be_bytes()
        );
    }

    #[test]
    #[ignore = "manual hardware integration test: requires real TPM hardware and suitable local key provisioning"]
    fn manual_hardware_integration_restricted_rsa_sign_end_to_end() {
        let handle = TpmHandle(0x8100_0002);
        let mut device = TpmDevice::new(LinuxTpmDevice::new("/dev/tpmrm0"));

        let public = device.read_public(handle).unwrap();
        assert_eq!(public.public_area.object_type, TpmPublicObjectType::Rsa);

        let message = b"lifegraph real TPM end-to-end verification";
        let hashed = device
            .hash(&TpmHashParams {
                data: message.to_vec(),
                hash_algorithm: TpmNameAlgorithm::Sha1,
                hierarchy: TPM_RH_OWNER,
            })
            .unwrap();
        assert_eq!(hashed.validation.tag, TPM_ST_HASHCHECK);
        assert_eq!(hashed.validation.hierarchy, TPM_RH_OWNER);

        let signature = device
            .sign_raw_with_password_auth(
                &TpmSignCommandParams {
                    key_handle: handle,
                    digest: hashed.digest.clone(),
                    scheme: TpmSignatureScheme {
                        scheme: 0x0014,
                        hash_algorithm: Some(TpmNameAlgorithm::Sha1),
                    },
                    validation_ticket_tag: hashed.validation.tag,
                    validation_ticket_hierarchy: hashed.validation.hierarchy,
                    validation_digest: hashed.validation.digest.clone(),
                },
                &TpmPasswordAuthSession {
                    auth_value: Vec::new(),
                    session_attributes: 0,
                },
            )
            .unwrap();

        device
            .verify_signature(handle, &hashed.digest, &signature)
            .unwrap();
    }
}
