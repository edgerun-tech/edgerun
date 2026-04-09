use edgerun_core::crypto::signature_input;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

mod tss2_esapi;

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
pub const TPM_CC_CREATE_PRIMARY: u32 = 0x0000_0131;
pub const TPM_CC_FLUSH_CONTEXT: u32 = 0x0000_0165;
pub const TPM_CC_STARTUP: u32 = 0x0000_0144;
pub const TPM_CC_SHUTDOWN: u32 = 0x0000_0145;
pub const TPM_SU_CLEAR: u16 = 0x0000;
pub const TPM_SU_STATE: u16 = 0x0001;
pub const TPM_PERSISTENT_FIRST: u32 = 0x8100_0000;
pub const TPM_SE_HMAC: u8 = 0x00;
pub const TPM_ECC_NIST_P256: u16 = 0x0003;
pub const TPM_ALG_ECDSA: u16 = 0x0018;
pub const TPM_ALG_SHA256: u16 = 0x000B;
pub const TPM_ALG_NULL: u16 = 0x0010;
pub const TPM_ALG_ECC: u16 = 0x0023;
// TPMA_OBJECT bits (from tpm2-tss tss2_tpm2_types.h)
pub const TPMA_OBJECT_FIXED_TPM: u32 =             0x0000_0002;
pub const TPMA_OBJECT_FIXED_PARENT: u32 =          0x0000_0010;
pub const TPMA_OBJECT_SENSITIVE_DATA_ORIGIN: u32 = 0x0000_0020;
pub const TPMA_OBJECT_USER_WITH_AUTH: u32 =        0x0000_0040;
pub const TPMA_OBJECT_NODA: u32 =                  0x0000_0400;
pub const TPMA_OBJECT_SIGN_ENCRYPT: u32 =          0x0004_0000;
pub const TPMA_OBJECT_DECRYPT: u32 =               0x0002_0000;
pub const TPMA_OBJECT_RESTRICTED: u32 =            0x0001_0000;
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
        // For persisted keys with userWithAuth, we need at least an empty password session
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

// ---------------------------------------------------------------------------
// CreatePrimary — create an ECDSA P-256 signing key under the owner hierarchy
// ---------------------------------------------------------------------------

/// Parameters for `CreatePrimary`.
#[derive(Clone, Debug)]
pub struct TpmCreatePrimaryParams {
    pub primary_handle: u32,       // e.g. TPM_RH_OWNER
    pub auth_value: Vec<u8>,        // optional password (empty = no auth)
    pub object_attributes: u32,
    pub ecc_curve: u16,             // TPM_ECC_NIST_P256 = 0x0003
    pub scheme: u16,                // TPM_ALG_ECDSA = 0x0018
    pub name_alg: u16,              // TPM_ALG_SHA256 = 0x000B
}

/// Result of a successful `CreatePrimary`.
#[derive(Clone, Debug)]
pub struct TpmCreatePrimaryResult {
    /// Virtual handle assigned by the TPM resource manager.
    pub object_handle: u32,
    /// Actual TPM handle (may differ from virtual handle due to RM mapping).
    pub tpm_handle: u32,
    /// The public key (x || y for ECDSA P-256, no 0x04 prefix).
    pub public_key: Vec<u8>,
    /// The TPM "name" of the key (algorithm || SHA-256 hash of public area).
    pub name: Vec<u8>,
    /// Raw public area bytes for reference.
    pub public_area_raw: Vec<u8>,
    /// Creation data (for future sealing/attestation).
    pub creation_data: Vec<u8>,
    /// Creation hash.
    pub creation_hash: Vec<u8>,
    /// Creation ticket.
    pub creation_ticket_tag: u16,
    pub creation_ticket_hierarchy: u32,
    pub creation_ticket_digest: Vec<u8>,
}

/// Result of creating and persisting a TPM key.
#[derive(Clone, Debug)]
pub struct TpmProvisionedKey {
    /// Persistent handle where the key was saved (e.g. 0x81000001).
    pub persistent_handle: u32,
    /// ECDSA P-256 public key (64 bytes: x || y).
    pub public_key_bytes: Vec<u8>,
    /// TPM name of the key.
    pub name: Vec<u8>,
}

// ---------------------------------------------------------------------------
// TPM2_Startup
// ---------------------------------------------------------------------------

/// Builds a `TPM2_Startup` command.
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

impl<T: TpmTransport> TpmDevice<T> {
    /// Sends a TPM2_Startup command to initialize the TPM.
    /// 
    /// This must be called once after system boot before any other TPM commands.
    /// Use `TPM_SU_CLEAR` for a normal startup, or `TPM_SU_STATE` to restore saved state.
    pub fn startup(&mut self, startup_type: u16) -> Result<(), TpmError> {
        let cmd = build_startup_command(startup_type);
        match self.transmit_command(&cmd) {
            Ok(_) => Ok(()),
            Err(TpmError::TpmResponseCode(0x120)) => {
                // TPM_RC_INITIALIZE - already started, ignore
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Creates a primary ECDSA P-256 signing key and persists it.
    ///
    /// Returns the persistent handle and public key bytes.
    ///
    /// Uses raw TSS2 ESAPI calls (tss-esapi-sys) to avoid pulling in the
    /// full tss-esapi crate which depends on regex for TCTI string parsing.
    pub fn create_ecdsa_p256_signing_key(
        &mut self,
        persistent_handle: u32,
    ) -> Result<TpmProvisionedKey, TpmError> {
        use crate::tss2_esapi::*;

        // Initialize TCTI for /dev/tpmrm0
        let tcti_name = b"device:/dev/tpmrm0\0";
        let mut tcti_ctx: *mut TSS2_TCTI_CONTEXT = std::ptr::null_mut();
        let rc = unsafe { Tss2_TctiLdr_Initialize(tcti_name.as_ptr().cast(), &mut tcti_ctx) };
        if rc != 0 {
            return Err(TpmError::Provider(format!(
                "Tss2_TctiLdr_Initialize failed: 0x{rc:08x}"
            )));
        }

        // Initialize ESYS context
        let mut esys_ctx: *mut ESYS_CONTEXT = std::ptr::null_mut();
        let rc = unsafe { Esys_Initialize(&mut esys_ctx, tcti_ctx, std::ptr::null_mut()) };
        if rc != 0 {
            unsafe { Tss2_TctiLdr_Finalize(&mut tcti_ctx) };
            return Err(TpmError::Provider(format!(
                "Esys_Initialize failed: 0x{rc:08x}"
            )));
        }

        // Startup TPM
        let rc = unsafe { Esys_Startup(esys_ctx, 0x0000 /* TPM2_SU_CLEAR */) };
        if rc != 0 && rc != 0x0000_0120 /* TPM_RC_INITIALIZE */ {
            unsafe { Esys_Finalize(&mut esys_ctx); Tss2_TctiLdr_Finalize(&mut tcti_ctx) };
            return Err(TpmError::Provider(format!(
                "Esys_Startup failed: 0x{rc:08x}"
            )));
        }

        // Build TPMT_PUBLIC template for ECDSA P-256
        let object_attributes = TPMA_OBJECT_SIGN_ENCRYPT
            | TPMA_OBJECT_FIXED_TPM
            | TPMA_OBJECT_FIXED_PARENT
            | TPMA_OBJECT_SENSITIVE_DATA_ORIGIN
            | TPMA_OBJECT_USER_WITH_AUTH
            | TPMA_OBJECT_NODA;

        let mut public: TPM2B_PUBLIC = unsafe { std::mem::zeroed() };
        public.publicArea.type_ = TPM2_ALG_ECC;
        public.publicArea.nameAlg = TPM2_ALG_SHA256;
        public.publicArea.objectAttributes = object_attributes;
        public.publicArea.parameters.eccDetail.scheme.scheme = TPM2_ALG_ECDSA;
        public.publicArea.parameters.eccDetail.scheme.hashAlg = TPM2_ALG_SHA256;
        public.publicArea.parameters.eccDetail.symmetric.algorithm = TPM2_ALG_NULL;
        public.publicArea.parameters.eccDetail.kdf.scheme = TPM2_ALG_NULL;
        public.publicArea.parameters.eccDetail.curveID = TPM2_ECC_NIST_P256;

        let in_sensitive: TPM2B_SENSITIVE_CREATE = unsafe { std::mem::zeroed() };
        let outside_info: TPM2B_DATA = unsafe { std::mem::zeroed() };
        let creation_pcr: TPML_PCR_SELECTION = unsafe { std::mem::zeroed() };

        let mut key_handle: ESYS_TR = ESYS_TR_NONE;
        let mut out_public: *mut TPM2B_PUBLIC = std::ptr::null_mut();

        let rc = unsafe {
            Esys_CreatePrimary(
                esys_ctx,
                TPM2_RH_OWNER as ESYS_TR,
                ESYS_TR_PASSWORD,
                ESYS_TR_NONE,
                ESYS_TR_NONE,
                &in_sensitive,
                &public,
                &outside_info,
                &creation_pcr,
                &mut key_handle,
                &mut out_public,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if rc != 0 {
            unsafe { Esys_Finalize(&mut esys_ctx); Tss2_TctiLdr_Finalize(&mut tcti_ctx) };
            return Err(TpmError::Provider(format!(
                "Esys_CreatePrimary failed: 0x{rc:08x}"
            )));
        }

        // Scan for an available persistent handle using raw TPM reads
        let mut available_handle = persistent_handle;
        {
            let mut check_device = TpmDevice::new(LinuxTpmDevice::new("/dev/tpmrm0"));
            for h in (0x81000001u32..=0x810000FF).step_by(1) {
                if check_device.read_public(TpmHandle(h)).is_err() {
                    available_handle = h;
                    break;
                }
            }
        }

        // Persist the key
        let mut new_handle: ESYS_TR = ESYS_TR_NONE;
        let rc = unsafe {
            Esys_EvictControl(
                esys_ctx,
                TPM2_RH_OWNER as ESYS_TR,
                key_handle,
                ESYS_TR_PASSWORD,
                ESYS_TR_NONE,
                ESYS_TR_NONE,
                available_handle,
                &mut new_handle,
            )
        };
        if rc != 0 {
            unsafe { Esys_FlushContext(esys_ctx, key_handle) };
            unsafe { Esys_Finalize(&mut esys_ctx); Tss2_TctiLdr_Finalize(&mut tcti_ctx) };
            return Err(TpmError::Provider(format!(
                "Esys_EvictControl failed: 0x{rc:08x}"
            )));
        }

        // Extract public key bytes (x || y) from out_public
        let public_key_bytes = if out_public.is_null() {
            return Err(TpmError::Protocol("null out_public from CreatePrimary".into()));
        } else {
            let pub_ref = unsafe { &*out_public };
            // SAFETY: We created an ECC key, so the union contains ecc variant
            let unique = unsafe { &pub_ref.publicArea.unique.ecc };
            let x_bytes = &unique.x.buffer[..unique.x.size as usize];
            let y_bytes = &unique.y.buffer[..unique.y.size as usize];
            let mut bytes = Vec::with_capacity(64);
            bytes.extend_from_slice(x_bytes);
            bytes.extend_from_slice(y_bytes);
            bytes
        };

        // Cleanup
        unsafe { Esys_FlushContext(esys_ctx, key_handle) };
        unsafe { Esys_Finalize(&mut esys_ctx); Tss2_TctiLdr_Finalize(&mut tcti_ctx) };

        Ok(TpmProvisionedKey {
            persistent_handle: available_handle,
            public_key_bytes,
            name: vec![],
        })
    }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

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
        let sig = sign_record_with_tpm(&key, "edgerun:v0:sig:test", &[9u8; 32]).unwrap();
        let expected_input = signature_input_for_record("edgerun:v0:sig:test", &[9u8; 32]);
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
            "edgerun:v0:sig:test",
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
            "edgerun:v0:sig:test",
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

        let message = b"edgerun real TPM end-to-end verification";
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

    // =========================================================================
    // Additional comprehensive tests
    // =========================================================================

    // --- 1. TPM command parsing and response parsing ---

    #[test]
    fn command_header_encoding_roundtrip() {
        let header = TpmCommandHeader {
            tag: TPM_ST_NO_SESSIONS,
            size: 20,
            command_code: TPM_CC_SIGN,
        };
        let mut out = Vec::new();
        encode_command_header(header, &mut out);
        assert_eq!(out.len(), 10);
        assert_eq!(&out[0..2], &TPM_ST_NO_SESSIONS.to_be_bytes());
        assert_eq!(&out[2..6], &20u32.to_be_bytes());
        assert_eq!(&out[6..10], &TPM_CC_SIGN.to_be_bytes());
    }

    #[test]
    fn command_header_encoding_sessions_tag() {
        let header = TpmCommandHeader {
            tag: TPM_ST_SESSIONS,
            size: 30,
            command_code: TPM_CC_POLICY_PCR,
        };
        let mut out = Vec::new();
        encode_command_header(header, &mut out);
        assert_eq!(&out[0..2], &TPM_ST_SESSIONS.to_be_bytes());
        assert_eq!(&out[2..6], &30u32.to_be_bytes());
        assert_eq!(&out[6..10], &TPM_CC_POLICY_PCR.to_be_bytes());
    }

    #[test]
    fn response_header_too_short_error() {
        let short = [0x80, 0x01, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00];
        let err = parse_response_header(&short).unwrap_err();
        assert!(matches!(err, TpmError::Protocol(_)));
        assert!(err.to_string().contains("10 bytes"));
    }

    #[test]
    fn response_header_empty_error() {
        let err = parse_response_header(&[]).unwrap_err();
        assert!(matches!(err, TpmError::Protocol(_)));
    }

    #[test]
    fn response_header_size_mismatch_error() {
        let mut resp = Vec::new();
        resp.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        resp.extend_from_slice(&20u32.to_be_bytes());
        resp.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        resp.extend_from_slice(&[0u8; 7]);
        let err = ensure_success_response(&resp).unwrap_err();
        assert!(matches!(err, TpmError::Protocol(_)));
        assert!(err.to_string().contains("size mismatch"));
    }

    #[test]
    fn response_header_tpm_error_code() {
        let mut resp = Vec::new();
        resp.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        resp.extend_from_slice(&10u32.to_be_bytes());
        resp.extend_from_slice(&0x0000_0100u32.to_be_bytes());
        let err = ensure_success_response(&resp).unwrap_err();
        assert!(matches!(err, TpmError::TpmResponseCode(0x0000_0100)));
    }

    #[test]
    fn response_header_various_tags() {
        let tags = [0x8001u16, 0x8002u16, 0x8024u16, 0xFFFFu16];
        for tag in tags {
            let bytes = [
                (tag >> 8) as u8,
                tag as u8,
                0x00, 0x00, 0x00, 0x0A,
                0x00, 0x00, 0x00, 0x00,
            ];
            let header = parse_response_header(&bytes).unwrap();
            assert_eq!(header.tag, tag);
        }
    }

    // --- 2. ReadPublic command/response extraction ---

    #[test]
    fn read_public_command_size_field_correct() {
        let cmd = build_read_public_command(TpmHandle(0x4000_0001));
        let size = u32::from_be_bytes(cmd[2..6].try_into().unwrap());
        assert_eq!(size as usize, cmd.len());
    }

    #[test]
    fn read_public_response_empty_name_and_qualified_name() {
        let mut public = Vec::new();
        // RSA type
        public.extend_from_slice(&0x0001u16.to_be_bytes());
        // SHA256 name alg
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        // object attributes + auth_policy (empty)
        public.extend_from_slice(&0x00030072u32.to_be_bytes());
        public.extend_from_slice(&0u16.to_be_bytes());
        // sym (NULL)
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        // scheme (NULL)
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        // key bits + exponent
        public.extend_from_slice(&2048u16.to_be_bytes());
        public.extend_from_slice(&0u32.to_be_bytes());
        // unique (empty RSA key)
        public.extend_from_slice(&0u16.to_be_bytes());

        let mut resp = Vec::new();
        resp.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        let total = 10 + 2 + public.len() + 2 + 0 + 2 + 0;
        resp.extend_from_slice(&(total as u32).to_be_bytes());
        resp.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        resp.extend_from_slice(&(public.len() as u16).to_be_bytes());
        resp.extend_from_slice(&public);
        resp.extend_from_slice(&0u16.to_be_bytes());
        resp.extend_from_slice(&0u16.to_be_bytes());

        let parsed = parse_read_public_response(&resp).unwrap();
        assert_eq!(parsed.public_area.object_type, TpmPublicObjectType::Rsa);
        assert_eq!(parsed.name, Vec::<u8>::new());
        assert_eq!(parsed.qualified_name, Vec::<u8>::new());
    }

    #[test]
    fn read_public_response_rsa_metadata() {
        let mut public = Vec::new();
        public.extend_from_slice(&0x0001u16.to_be_bytes());
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        public.extend_from_slice(&0x00030072u32.to_be_bytes());
        public.extend_from_slice(&0u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        public.extend_from_slice(&0x0014u16.to_be_bytes());
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        public.extend_from_slice(&2048u16.to_be_bytes());
        public.extend_from_slice(&0u32.to_be_bytes());
        let unique = vec![0xCC; 256];
        public.extend_from_slice(&(unique.len() as u16).to_be_bytes());
        public.extend_from_slice(&unique);

        let mut resp = Vec::new();
        let total = 10 + 2 + public.len() + 2 + 4 + 2 + 4;
        resp.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        resp.extend_from_slice(&(total as u32).to_be_bytes());
        resp.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        resp.extend_from_slice(&(public.len() as u16).to_be_bytes());
        resp.extend_from_slice(&public);
        resp.extend_from_slice(&4u16.to_be_bytes());
        resp.extend_from_slice(&[1, 2, 3, 4]);
        resp.extend_from_slice(&4u16.to_be_bytes());
        resp.extend_from_slice(&[5, 6, 7, 8]);

        let parsed = parse_read_public_response(&resp).unwrap();
        assert_eq!(parsed.public_area.object_type, TpmPublicObjectType::Rsa);
        assert_eq!(parsed.public_area.name_algorithm, TpmNameAlgorithm::Sha256);
        assert_eq!(parsed.public_area.key_bits, Some(2048));
        assert_eq!(parsed.public_area.unique.len(), 256);
        assert_eq!(
            infer_signature_algorithm(&parsed.public_area),
            Some(TpmSignatureAlgorithm::RsaPssSha256)
        );
    }

    // --- 3. Sign operation parsing ---

    #[test]
    fn parse_sign_response_with_sessions_tag_and_auth_area() {
        let mut response = Vec::new();
        let mut params = Vec::new();
        params.extend_from_slice(&0x0018u16.to_be_bytes());
        params.extend_from_slice(&0x000Bu16.to_be_bytes());
        params.extend_from_slice(&2u16.to_be_bytes());
        params.extend_from_slice(&[1, 2]);
        params.extend_from_slice(&2u16.to_be_bytes());
        params.extend_from_slice(&[3, 4]);

        response.extend_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
        let total = 10 + 4 + params.len();
        response.extend_from_slice(&(total as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&(params.len() as u32).to_be_bytes());
        response.extend_from_slice(&params);

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
    fn parse_sign_response_opaque_scheme() {
        let mut response = Vec::new();
        let mut body = Vec::new();
        body.extend_from_slice(&0x9999u16.to_be_bytes());
        body.extend_from_slice(&[0xAB, 0xCD]);
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&body);

        let parsed = parse_sign_response(&response).unwrap();
        assert_eq!(
            parsed,
            TpmParsedSignature::Opaque {
                scheme: 0x9999,
                bytes: vec![0xAB, 0xCD],
            }
        );
    }

    #[test]
    fn parse_sign_response_trailing_bytes_error() {
        let mut response = Vec::new();
        let mut body = Vec::new();
        body.extend_from_slice(&0x0018u16.to_be_bytes());
        body.extend_from_slice(&0x000Bu16.to_be_bytes());
        body.extend_from_slice(&2u16.to_be_bytes());
        body.extend_from_slice(&[1, 2]);
        body.extend_from_slice(&2u16.to_be_bytes());
        body.extend_from_slice(&[3, 4]);
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&((10 + body.len() + 3) as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&body);
        response.extend_from_slice(&[0xFF, 0xFF, 0xFF]);

        let err = parse_sign_response(&response).unwrap_err();
        assert!(err.to_string().contains("trailing bytes"));
    }

    #[test]
    fn sign_command_with_no_sessions_tag_has_correct_body() {
        let cmd = build_sign_command(&TpmSignCommandParams {
            key_handle: TpmHandle(0x8100_0001),
            digest: vec![0xAA; 32],
            scheme: default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::RsaPkcs1v15Sha256),
            validation_ticket_tag: TPM_ST_HASHCHECK,
            validation_ticket_hierarchy: TPM_RH_NULL,
            validation_digest: vec![],
        });
        assert_eq!(&cmd[6..10], &TPM_CC_SIGN.to_be_bytes());
        assert_eq!(&cmd[14..16], &32u16.to_be_bytes());
        assert_eq!(&cmd[16..48], &[0xAA; 32]);
        // scheme 0x0014, hash Sha256 (0x000B)
        assert_eq!(&cmd[48..50], &0x0014u16.to_be_bytes());
        assert_eq!(&cmd[50..52], &0x000Bu16.to_be_bytes());
    }

    // --- 4. Hash operation parsing ---

    #[test]
    fn hash_command_empty_data() {
        let cmd = build_hash_command(&TpmHashParams {
            data: vec![],
            hash_algorithm: TpmNameAlgorithm::Sha256,
            hierarchy: TPM_RH_OWNER,
        });
        assert_eq!(&cmd[10..12], &0u16.to_be_bytes());
        assert_eq!(&cmd[12..14], &0x000Bu16.to_be_bytes());
    }

    #[test]
    fn hash_response_malformed_digest_length_error() {
        let body = [
            0x00, 0x10, // digest length = 16, but only 5 bytes follow
            0x01, 0x02, 0x03, 0x04, 0x05,
            0x80, 0x24, // tag
            0x00, 0x00, 0x00, 0x00, // hierarchy
            0x00, 0x00, // ticket digest length
        ];
        let total = 10 + body.len();
        let mut response = Vec::new();
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&(total as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&body);

        let err = parse_hash_response(&response).unwrap_err();
        assert!(err.to_string().contains("length exceeds buffer"));
    }

    #[test]
    fn hash_response_missing_ticket_error() {
        let body = [
            0x00, 0x04, // digest length = 4
            0x01, 0x02, 0x03, 0x04,
            0x80, // only 1 byte of tag, not enough for u16
        ];
        let total = 10 + body.len();
        let mut response = Vec::new();
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&(total as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&body);

        let err = parse_hash_response(&response).unwrap_err();
        assert!(err.to_string().contains("missing u16"));
    }

    #[test]
    fn hash_response_trailing_bytes_error() {
        let body = [
            0x00, 0x04, // digest length = 4
            0x01, 0x02, 0x03, 0x04,
            0x80, 0x24, // tag
            0x00, 0x00, 0x00, 0x00, // hierarchy
            0x00, 0x03, // ticket digest length
            0x05, 0x06, 0x07,
        ];
        let total = 10 + body.len() + 1; // +1 for trailing
        let mut response = Vec::new();
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&(total as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&body);
        response.push(0xFF);

        let err = parse_hash_response(&response).unwrap_err();
        assert!(err.to_string().contains("trailing bytes"));
    }

    // --- 5. VerifySignature parsing ---

    #[test]
    fn verify_signature_command_ecc() {
        let cmd = build_verify_signature_command(
            TpmHandle(0x8100_0003),
            &[0xDD; 32],
            &TpmParsedSignature::Ecc {
                scheme: 0x0018,
                hash_algorithm: TpmNameAlgorithm::Sha256,
                r: vec![1, 2, 3],
                s: vec![4, 5, 6],
            },
        );
        assert_eq!(&cmd[0..2], &TPM_ST_NO_SESSIONS.to_be_bytes());
        assert_eq!(&cmd[6..10], &TPM_CC_VERIFY_SIGNATURE.to_be_bytes());
        assert_eq!(&cmd[10..14], &0x8100_0003u32.to_be_bytes());
        assert_eq!(&cmd[14..16], &32u16.to_be_bytes());
        // encoded signature: body starts at offset 10, body = handle(4) + digest_len(2) + digest(32) + signature
        let sig_abs_offset = 10 + 4 + 2 + 32;
        assert_eq!(&cmd[sig_abs_offset..sig_abs_offset + 2], &0x0018u16.to_be_bytes());
    }

    #[test]
    fn verify_signature_command_opaque() {
        let cmd = build_verify_signature_command(
            TpmHandle(0x8100_0003),
            &[0xDD; 32],
            &TpmParsedSignature::Opaque {
                scheme: 0x9999,
                bytes: vec![0xAA, 0xBB],
            },
        );
        let sig_abs_offset = 10 + 4 + 2 + 32;
        assert_eq!(&cmd[sig_abs_offset..sig_abs_offset + 2], &0x9999u16.to_be_bytes());
        assert_eq!(&cmd[sig_abs_offset + 2..sig_abs_offset + 4], &[0xAA, 0xBB]);
    }

    // --- 6. StartAuthSession parsing ---

    #[test]
    fn parse_start_auth_session_response_too_short() {
        let body = [
            0x03, 0x00, 0x00, 0x00, // session_handle
            0x00, // only 1 byte, need 2 for nonce length
        ];
        let total = 10 + body.len();
        let mut resp = Vec::new();
        resp.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        resp.extend_from_slice(&(total as u32).to_be_bytes());
        resp.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        resp.extend_from_slice(&body);

        let err = parse_start_auth_session_response(&resp).unwrap_err();
        assert!(err.to_string().contains("missing u16"));
    }

    #[test]
    fn parse_start_auth_session_response_trailing() {
        let body = [
            0x03, 0x00, 0x00, 0x00, // session_handle
            0x00, 0x04, // nonce length
            0x09, 0x08, 0x07, 0x06, // nonce
        ];
        let total = 10 + body.len() + 1;
        let mut resp = Vec::new();
        resp.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        resp.extend_from_slice(&(total as u32).to_be_bytes());
        resp.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        resp.extend_from_slice(&body);
        resp.push(0xFF);

        let err = parse_start_auth_session_response(&resp).unwrap_err();
        assert!(err.to_string().contains("trailing bytes"));
    }

    #[test]
    fn start_auth_session_with_hmac_type() {
        let cmd = build_start_auth_session_command(&TpmStartAuthSessionParams {
            tpm_key: TpmHandle(0x8100_0001),
            bind: TpmHandle(0x4000_0001),
            nonce_caller: vec![0x11; 16],
            session_type: TpmSessionType::Hmac,
            symmetric: TpmSymmetricDefinition::Null,
            auth_hash: TpmNameAlgorithm::Sha384,
        });
        assert_eq!(&cmd[0..2], &TPM_ST_NO_SESSIONS.to_be_bytes());
        assert_eq!(&cmd[6..10], &TPM_CC_START_AUTH_SESSION.to_be_bytes());
        // session type byte: after header(10) + tpm_key(4) + bind(4) + nonce_len(2) + nonce(16) = offset 36
        assert_eq!(cmd[36], 0x00); // HMAC
    }

    #[test]
    fn start_auth_session_with_trial_type() {
        let cmd = build_start_auth_session_command(&TpmStartAuthSessionParams {
            tpm_key: TpmHandle(TPM_RH_NULL),
            bind: TpmHandle(TPM_RH_NULL),
            nonce_caller: vec![],
            session_type: TpmSessionType::Trial,
            symmetric: TpmSymmetricDefinition::Null,
            auth_hash: TpmNameAlgorithm::Sha512,
        });
        // session type byte: after header(10) + tpm_key(4) + bind(4) + nonce_len(2) + nonce(0) = offset 20
        assert_eq!(cmd[20], 0x03); // Trial
    }

    #[test]
    fn start_auth_session_empty_nonce() {
        let cmd = build_start_auth_session_command(&TpmStartAuthSessionParams {
            tpm_key: TpmHandle(TPM_RH_NULL),
            bind: TpmHandle(TPM_RH_NULL),
            nonce_caller: vec![],
            session_type: TpmSessionType::Policy,
            symmetric: TpmSymmetricDefinition::Null,
            auth_hash: TpmNameAlgorithm::Sha256,
        });
        // nonce length at offset 18
        assert_eq!(&cmd[18..20], &0u16.to_be_bytes());
    }

    // --- 7. PolicyPCR, PolicyAuthorize, PolicyCommandCode parsing ---

    #[test]
    fn build_policy_pcr_command_body_structure() {
        let cmd = build_policy_pcr_command(&TpmPolicyPcrParams {
            session: TpmPolicySession {
                session_handle: 0x0300_0000,
                session_attributes: 0x01,
            },
            pcr_digest: vec![0xBB; 32],
            selection: TpmPcrSelection {
                hash_algorithm: TpmNameAlgorithm::Sha256,
                pcrs: vec![0, 1, 7, 16],
            },
        });
        assert_eq!(&cmd[0..2], &TPM_ST_SESSIONS.to_be_bytes());
        assert_eq!(&cmd[6..10], &TPM_CC_POLICY_PCR.to_be_bytes());
    }

    #[test]
    fn build_policy_pcr_command_pcr_bitmap() {
        let cmd = build_policy_pcr_command(&TpmPolicyPcrParams {
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
        // bits are the last 3 bytes of the command
        // pcr 0 => bit 0, pcr 7 => bit 7 => 0x01 | 0x80 = 0x81
        let bits_start = cmd.len() - 3;
        assert_eq!(cmd[bits_start], 0x81); // bits for PCR 0 and 7
        assert_eq!(cmd[bits_start + 1], 0x00);
        assert_eq!(cmd[bits_start + 2], 0x00);
    }

    #[test]
    fn build_policy_authorize_command_all_fields() {
        let cmd = build_policy_authorize_command(&TpmPolicyAuthorizeParams {
            session: TpmPolicySession {
                session_handle: 0x0300_0001,
                session_attributes: 0x02,
            },
            approved_policy: vec![0x11, 0x22, 0x33, 0x44],
            policy_ref: vec![0x55],
            key_sign_name: vec![0x66, 0x77],
            check_ticket_hierarchy: TPM_RH_NULL,
            check_ticket_digest: vec![0x88, 0x99],
        });
        assert_eq!(&cmd[0..2], &TPM_ST_SESSIONS.to_be_bytes());
        assert_eq!(&cmd[6..10], &TPM_CC_POLICY_AUTHORIZE.to_be_bytes());
        assert_eq!(&cmd[10..14], &0x0300_0001u32.to_be_bytes());
    }

    #[test]
    fn build_policy_command_code_command_multiple_codes() {
        let codes = [TPM_CC_SIGN, TPM_CC_HASH, TPM_CC_READ_PUBLIC, TPM_CC_VERIFY_SIGNATURE, TPM_CC_POLICY_COMMAND_CODE];
        for code in codes {
            let cmd = build_policy_command_code_command(
                TpmPolicySession {
                    session_handle: 0x0300_0000,
                    session_attributes: 0,
                },
                code,
            );
            assert_eq!(&cmd[0..2], &TPM_ST_SESSIONS.to_be_bytes());
            assert_eq!(&cmd[6..10], &TPM_CC_POLICY_COMMAND_CODE.to_be_bytes());
        }
    }

    #[test]
    fn build_policy_command_code_auth_area_structure() {
        let cmd = build_policy_command_code_command(
            TpmPolicySession {
                session_handle: 0x0300_0002,
                session_attributes: 0x04,
            },
            TPM_CC_SIGN,
        );
        // After header (10) + session_handle (4) + auth_size (4)
        let auth_size = u32::from_be_bytes(cmd[14..18].try_into().unwrap()) as usize;
        let auth_start = 18;
        assert_eq!(auth_start + auth_size + 4, cmd.len()); // auth + command_code
        assert_eq!(&cmd[auth_start..auth_start + 4], &0x0300_0002u32.to_be_bytes());
    }

    // --- 8. Response header roundtrips ---

    #[test]
    fn response_header_roundtrip_sessions_tag() {
        let bytes = [0x80, 0x02, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00];
        let header = parse_response_header(&bytes).unwrap();
        assert_eq!(header.tag, TPM_ST_SESSIONS);
        assert_eq!(header.size, 16);
        assert_eq!(header.response_code, TPM_RC_SUCCESS);
    }

    #[test]
    fn response_header_roundtrip_hashcheck_tag() {
        let bytes = [0x80, 0x24, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x01, 0x00];
        let header = parse_response_header(&bytes).unwrap();
        assert_eq!(header.tag, TPM_ST_HASHCHECK);
        assert_eq!(header.size, 14);
        assert_eq!(header.response_code, 0x100);
    }

    #[test]
    fn response_header_all_zeros() {
        let bytes = [0u8; 10];
        let header = parse_response_header(&bytes).unwrap();
        assert_eq!(header.tag, 0);
        assert_eq!(header.size, 0);
        assert_eq!(header.response_code, 0);
    }

    #[test]
    fn response_header_all_ones() {
        let bytes = [0xFFu8; 10];
        let header = parse_response_header(&bytes).unwrap();
        assert_eq!(header.tag, 0xFFFF);
        assert_eq!(header.size, 0xFFFFFFFF);
        assert_eq!(header.response_code, 0xFFFFFFFF);
    }

    // --- 9. ECC P-256 metadata extraction ---

    #[test]
    fn ecc_p256_unique_is_concatenation_of_x_and_y() {
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
        let x = vec![0x11; 32];
        let y = vec![0x22; 32];
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&x);
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&y);

        let info = parse_public_area(&public).unwrap();
        assert_eq!(info.unique.len(), 64);
        assert_eq!(&info.unique[0..32], &x);
        assert_eq!(&info.unique[32..64], &y);
    }

    #[test]
    fn ecc_p384_curve_mapping() {
        let mut public = Vec::new();
        public.extend_from_slice(&0x0023u16.to_be_bytes()); // type = ECC
        public.extend_from_slice(&0x000Cu16.to_be_bytes()); // name_alg = SHA384
        public.extend_from_slice(&0x00030072u32.to_be_bytes()); // object_attributes
        public.extend_from_slice(&0u16.to_be_bytes()); // auth_policy len = 0
        public.extend_from_slice(&0x0010u16.to_be_bytes()); // sym_def = NULL
        public.extend_from_slice(&0x0010u16.to_be_bytes()); // ecc_scheme = NULL
        public.extend_from_slice(&0x0004u16.to_be_bytes()); // curve = NistP384
        public.extend_from_slice(&0x0010u16.to_be_bytes()); // kdf_scheme = NULL
        let x = vec![0x33; 48];
        let y = vec![0x44; 48];
        public.extend_from_slice(&48u16.to_be_bytes());
        public.extend_from_slice(&x);
        public.extend_from_slice(&48u16.to_be_bytes());
        public.extend_from_slice(&y);

        let info = parse_public_area(&public).unwrap();
        assert_eq!(info.curve, Some(TpmEccCurve::NistP384));
        assert_eq!(
            infer_signature_algorithm(&info),
            Some(TpmSignatureAlgorithm::EcdsaP384Sha384)
        );
    }

    #[test]
    fn ecc_curve25519_curve_mapping() {
        let mut public = Vec::new();
        public.extend_from_slice(&0x0023u16.to_be_bytes());
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        public.extend_from_slice(&0x00030072u32.to_be_bytes());
        public.extend_from_slice(&0u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        public.extend_from_slice(&0x0040u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        let x = vec![0x55; 32];
        let y = vec![0x66; 32];
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&x);
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&y);

        let info = parse_public_area(&public).unwrap();
        assert_eq!(info.curve, Some(TpmEccCurve::Curve25519));
        assert_eq!(
            infer_signature_algorithm(&info),
            Some(TpmSignatureAlgorithm::Eddsa)
        );
    }

    #[test]
    fn ecc_unknown_curve() {
        let curve = map_ecc_curve(0x9999);
        assert_eq!(curve, TpmEccCurve::Unknown(0x9999));
    }

    #[test]
    fn infer_signature_algorithm_returns_none_for_keyed_hash() {
        let info = TpmPublicAreaInfo {
            object_type: TpmPublicObjectType::KeyedHash,
            name_algorithm: TpmNameAlgorithm::Sha256,
            object_attributes: 0,
            auth_policy: vec![],
            parameters: vec![],
            unique: vec![0x01],
            curve: None,
            key_bits: None,
        };
        assert_eq!(infer_signature_algorithm(&info), None);
    }

    #[test]
    fn infer_signature_algorithm_returns_none_for_sym_cipher() {
        let info = TpmPublicAreaInfo {
            object_type: TpmPublicObjectType::SymCipher,
            name_algorithm: TpmNameAlgorithm::Sha256,
            object_attributes: 0,
            auth_policy: vec![],
            parameters: vec![],
            unique: vec![0x01],
            curve: None,
            key_bits: None,
        };
        assert_eq!(infer_signature_algorithm(&info), None);
    }

    #[test]
    fn infer_signature_algorithm_returns_none_for_unknown_object_type() {
        let info = TpmPublicAreaInfo {
            object_type: TpmPublicObjectType::Unknown(0x9999),
            name_algorithm: TpmNameAlgorithm::Sha256,
            object_attributes: 0,
            auth_policy: vec![],
            parameters: vec![],
            unique: vec![],
            curve: None,
            key_bits: None,
        };
        assert_eq!(infer_signature_algorithm(&info), None);
    }

    // --- 10. Error types and their display implementations ---

    #[test]
    fn tpm_error_display_provider() {
        let err = TpmError::Provider("custom error".to_string());
        assert_eq!(err.to_string(), "custom error");
    }

    #[test]
    fn tpm_error_display_unsupported_algorithm() {
        let err = TpmError::UnsupportedAlgorithm(TpmSignatureAlgorithm::EcdsaP256Sha256);
        assert!(err.to_string().contains("unsupported TPM signature algorithm"));
        assert!(err.to_string().contains("EcdsaP256Sha256"));
    }

    #[test]
    fn tpm_error_display_unsupported_algorithm_opaque() {
        let err = TpmError::UnsupportedAlgorithm(TpmSignatureAlgorithm::Opaque("custom".to_string()));
        assert!(err.to_string().contains("unsupported TPM signature algorithm"));
        assert!(err.to_string().contains("Opaque"));
    }

    #[test]
    fn tpm_error_display_unsupported_public_area() {
        let err = TpmError::UnsupportedPublicArea;
        assert_eq!(err.to_string(), "unsupported TPM public area");
    }

    #[test]
    fn tpm_error_display_io() {
        let err = TpmError::Io("failed to open".to_string());
        assert!(err.to_string().contains("TPM I/O error"));
        assert!(err.to_string().contains("failed to open"));
    }

    #[test]
    fn tpm_error_display_protocol() {
        let err = TpmError::Protocol("bad format".to_string());
        assert!(err.to_string().contains("TPM protocol error"));
        assert!(err.to_string().contains("bad format"));
    }

    #[test]
    fn tpm_error_display_tpm_response_code() {
        let err = TpmError::TpmResponseCode(0x0000_0100);
        assert!(err.to_string().contains("TPM returned response code"));
        assert!(err.to_string().contains("0x00000100"));
    }

    #[test]
    fn tpm_error_display_tpm_response_code_zero() {
        let err = TpmError::TpmResponseCode(0);
        assert!(err.to_string().contains("0x00000000"));
    }

    #[test]
    fn tpm_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(TpmError::Protocol("test".to_string()));
        assert!(err.to_string().contains("TPM protocol error"));
    }

    #[test]
    fn tpm_error_clone_and_debug_traits() {
        let err = TpmError::Protocol("clone test".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
        assert_eq!(format!("{:?}", err), format!("{:?}", cloned));
    }

    // --- 11. Edge cases: truncated data, malformed commands, buffer boundaries ---

    #[test]
    fn read_u16_at_buffer_boundary() {
        let buf = [0x12, 0x34];
        let mut cursor = 0usize;
        let val = read_u16(&buf, &mut cursor, "test").unwrap();
        assert_eq!(val, 0x1234);
        assert_eq!(cursor, 2);
    }

    #[test]
    fn read_u16_past_buffer_boundary() {
        let buf = [0x12];
        let mut cursor = 0usize;
        let err = read_u16(&buf, &mut cursor, "test").unwrap_err();
        assert!(err.to_string().contains("missing u16"));
    }

    #[test]
    fn read_u16_exactly_one_byte_short() {
        let buf = [0x12];
        let mut cursor = 0usize;
        let err = read_u16(&buf, &mut cursor, "boundary_test").unwrap_err();
        assert!(err.to_string().contains("missing u16 field boundary_test"));
    }

    #[test]
    fn read_u32_at_buffer_boundary() {
        let buf = [0x12, 0x34, 0x56, 0x78];
        let mut cursor = 0usize;
        let val = read_u32(&buf, &mut cursor, "test").unwrap();
        assert_eq!(val, 0x12345678);
        assert_eq!(cursor, 4);
    }

    #[test]
    fn read_u32_past_buffer_boundary() {
        let buf = [0x12, 0x34, 0x56];
        let mut cursor = 0usize;
        let err = read_u32(&buf, &mut cursor, "test").unwrap_err();
        assert!(err.to_string().contains("missing u32"));
    }

    #[test]
    fn read_u32_partial_at_end() {
        let buf = [0x12, 0x34, 0x56];
        let mut cursor = 0usize;
        let err = read_u32(&buf, &mut cursor, "partial").unwrap_err();
        assert!(err.to_string().contains("missing u32 field partial"));
    }

    #[test]
    fn read_tpm2b_zero_length() {
        let buf = [0x00, 0x00];
        let mut cursor = 0usize;
        let val = read_tpm2b(&buf, &mut cursor, "test").unwrap();
        assert!(val.is_empty());
        assert_eq!(cursor, 2);
    }

    #[test]
    fn read_tpm2b_length_exceeds_buffer() {
        let buf = [0x00, 0x10, 0x01, 0x02];
        let mut cursor = 0usize;
        let err = read_tpm2b(&buf, &mut cursor, "test").unwrap_err();
        assert!(err.to_string().contains("length exceeds buffer"));
    }

    #[test]
    fn read_tpm2b_at_exact_boundary() {
        let buf = [0x00, 0x02, 0xAA, 0xBB];
        let mut cursor = 0usize;
        let val = read_tpm2b(&buf, &mut cursor, "test").unwrap();
        assert_eq!(val, vec![0xAA, 0xBB]);
        assert_eq!(cursor, 4);
    }

    #[test]
    fn read_tpm2b_truncated_mid_length() {
        let buf = [0x00, 0x05, 0x01, 0x02];
        let mut cursor = 0usize;
        let err = read_tpm2b(&buf, &mut cursor, "truncated").unwrap_err();
        assert!(err.to_string().contains("length exceeds buffer"));
    }

    #[test]
    fn parse_response_header_exactly_10_bytes() {
        let bytes = [0x80, 0x01, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x00];
        assert!(parse_response_header(&bytes).is_ok());
    }

    #[test]
    fn parse_response_header_9_bytes() {
        let bytes = [0x80, 0x01, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00];
        assert!(parse_response_header(&bytes).is_err());
    }

    #[test]
    fn parse_read_public_response_truncated_public_area_length() {
        // Declared size matches buffer, but the tpm2b length says 5 bytes and only 2 follow
        let body = [
            0x00, 0x05, // public_area length = 5, but only 2 bytes
            0x01, 0x02,
        ];
        let total = 10 + body.len();
        let mut resp = Vec::new();
        resp.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        resp.extend_from_slice(&(total as u32).to_be_bytes());
        resp.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        resp.extend_from_slice(&body);

        let err = parse_read_public_response(&resp).unwrap_err();
        assert!(err.to_string().contains("length exceeds buffer"));
    }

    #[test]
    fn parse_sign_response_too_short_for_scheme() {
        let body = [0x00]; // only 1 byte, need 2 for scheme
        let total = 10 + body.len();
        let mut response = Vec::new();
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&(total as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&body);

        let err = parse_sign_response(&response).unwrap_err();
        assert!(err.to_string().contains("missing u16"));
    }

    #[test]
    fn parse_sign_response_ecc_missing_s() {
        let mut response = Vec::new();
        let mut body = Vec::new();
        body.extend_from_slice(&0x0018u16.to_be_bytes());
        body.extend_from_slice(&0x000Bu16.to_be_bytes());
        body.extend_from_slice(&2u16.to_be_bytes());
        body.extend_from_slice(&[1, 2]);
        // missing s length + s data
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&body);

        let err = parse_sign_response(&response).unwrap_err();
        assert!(err.to_string().contains("missing u16"));
    }

    #[test]
    fn parse_start_auth_session_response_missing_nonce_length() {
        let mut resp = Vec::new();
        resp.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        resp.extend_from_slice(&14u32.to_be_bytes());
        resp.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        resp.extend_from_slice(&0x0300_0000u32.to_be_bytes());
        // missing nonce_tpm length

        let err = parse_start_auth_session_response(&resp).unwrap_err();
        assert!(err.to_string().contains("missing u16"));
    }

    #[test]
    fn parse_hash_response_missing_validation_tag() {
        let mut response = Vec::new();
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&16u32.to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&4u16.to_be_bytes());
        response.extend_from_slice(&[1, 2, 3, 4]);
        // missing tag, hierarchy, digest

        let err = parse_hash_response(&response).unwrap_err();
        assert!(err.to_string().contains("missing u16"));
    }

    #[test]
    fn build_command_overflow_protection_size_field() {
        // Build a sign command and verify the size field matches actual length
        let cmd = build_sign_command(&TpmSignCommandParams {
            key_handle: TpmHandle(0x8100_0001),
            digest: vec![0xAA; 32],
            scheme: default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::EcdsaP256Sha256),
            validation_ticket_tag: TPM_ST_HASHCHECK,
            validation_ticket_hierarchy: TPM_RH_NULL,
            validation_digest: vec![0xBB; 8],
        });
        let declared_size = u32::from_be_bytes(cmd[2..6].try_into().unwrap());
        assert_eq!(declared_size as usize, cmd.len());
    }

    #[test]
    fn build_hash_command_size_matches_actual() {
        let cmd = build_hash_command(&TpmHashParams {
            data: vec![0xCC; 100],
            hash_algorithm: TpmNameAlgorithm::Sha512,
            hierarchy: TPM_RH_NULL,
        });
        let declared_size = u32::from_be_bytes(cmd[2..6].try_into().unwrap());
        assert_eq!(declared_size as usize, cmd.len());
    }

    // --- 12. All public types and their methods ---

    #[test]
    fn tpm_handle_clone_and_equality() {
        let h1 = TpmHandle(0x8100_0001);
        let h2 = h1.clone();
        assert_eq!(h1, h2);
        assert_eq!(h1.0, 0x8100_0001);
    }

    #[test]
    fn tpm_signature_algorithm_clone_and_equality() {
        let alg1 = TpmSignatureAlgorithm::EcdsaP256Sha256;
        let alg2 = alg1.clone();
        assert_eq!(alg1, alg2);
        let alg3 = TpmSignatureAlgorithm::Opaque("test".to_string());
        let alg4 = alg3.clone();
        assert_eq!(alg3, alg4);
    }

    #[test]
    fn tpm_assurance_level_clone_and_debug() {
        let levels = [
            TpmAssuranceLevel::SoftwareSimulated,
            TpmAssuranceLevel::DiscreteTpm,
            TpmAssuranceLevel::IntegratedTpm,
            TpmAssuranceLevel::Certified("cert".to_string()),
            TpmAssuranceLevel::Unknown,
        ];
        for level in &levels {
            let cloned = level.clone();
            assert_eq!(level, &cloned);
            assert!(!format!("{:?}", level).is_empty());
        }
    }

    #[test]
    fn tpm_key_info_clone_and_debug() {
        let info = TpmKeyInfo {
            key_name: "test-key".to_string(),
            algorithm: TpmSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![0x01, 0x02],
            attestation_blob: Some(vec![0x03]),
            assurance_level: TpmAssuranceLevel::DiscreteTpm,
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
        assert!(!format!("{:?}", info).is_empty());
    }

    #[test]
    fn tpm_command_header_copy_traits() {
        let h = TpmCommandHeader {
            tag: TPM_ST_NO_SESSIONS,
            size: 10,
            command_code: TPM_CC_READ_PUBLIC,
        };
        let h2 = h;
        let h3 = h;
        assert_eq!(h2, h3);
    }

    #[test]
    fn tpm_response_header_copy_traits() {
        let h = TpmResponseHeader {
            tag: TPM_ST_SESSIONS,
            size: 20,
            response_code: 0,
        };
        let h2 = h;
        let h3 = h;
        assert_eq!(h2, h3);
    }

    #[test]
    fn tpm_public_object_type_mapping_all_known() {
        assert_eq!(map_public_object_type(0x0001), TpmPublicObjectType::Rsa);
        assert_eq!(map_public_object_type(0x0023), TpmPublicObjectType::Ecc);
        assert_eq!(map_public_object_type(0x0008), TpmPublicObjectType::KeyedHash);
        assert_eq!(map_public_object_type(0x0025), TpmPublicObjectType::SymCipher);
        assert_eq!(map_public_object_type(0xFFFF), TpmPublicObjectType::Unknown(0xFFFF));
    }

    #[test]
    fn tpm_name_algorithm_encoding_and_mapping() {
        let pairs = [
            (TpmNameAlgorithm::Sha1, 0x0004u16),
            (TpmNameAlgorithm::Sha256, 0x000B),
            (TpmNameAlgorithm::Sha384, 0x000C),
            (TpmNameAlgorithm::Sha512, 0x000D),
            (TpmNameAlgorithm::Null, 0x0010),
        ];
        for (alg, enc) in pairs {
            assert_eq!(encode_name_algorithm(alg), enc);
            assert_eq!(map_name_algorithm(enc), alg);
        }
    }

    #[test]
    fn tpm_name_algorithm_unknown_roundtrip() {
        assert_eq!(encode_name_algorithm(TpmNameAlgorithm::Unknown(0x9999)), 0x9999);
        assert_eq!(map_name_algorithm(0x9999), TpmNameAlgorithm::Unknown(0x9999));
    }

    #[test]
    fn tpm_ecc_curve_encoding_all_known() {
        assert_eq!(map_ecc_curve(0x0003), TpmEccCurve::NistP256);
        assert_eq!(map_ecc_curve(0x0004), TpmEccCurve::NistP384);
        assert_eq!(map_ecc_curve(0x0040), TpmEccCurve::Curve25519);
    }

    #[test]
    fn tpm_session_type_values() {
        assert_eq!(TpmSessionType::Hmac as u8, 0x00);
        assert_eq!(TpmSessionType::Policy as u8, 0x01);
        assert_eq!(TpmSessionType::Trial as u8, 0x03);
    }

    #[test]
    fn tpm_symmetric_definition_null() {
        assert_eq!(encode_symmetric_definition(TpmSymmetricDefinition::Null), 0x0010u16.to_be_bytes());
    }

    #[test]
    fn tpm_public_area_info_clone() {
        let info = TpmPublicAreaInfo {
            object_type: TpmPublicObjectType::Ecc,
            name_algorithm: TpmNameAlgorithm::Sha256,
            object_attributes: 0x00030072,
            auth_policy: vec![0x01, 0x02],
            parameters: vec![0x03],
            unique: vec![0x04, 0x05],
            curve: Some(TpmEccCurve::NistP256),
            key_bits: None,
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn tpm_read_public_info_clone() {
        let info = TpmReadPublicInfo {
            public_area: TpmPublicAreaInfo {
                object_type: TpmPublicObjectType::Rsa,
                name_algorithm: TpmNameAlgorithm::Sha256,
                object_attributes: 0,
                auth_policy: vec![],
                parameters: vec![],
                unique: vec![],
                curve: None,
                key_bits: Some(2048),
            },
            public_area_raw: vec![0x01],
            name: vec![0x02],
            qualified_name: vec![0x03],
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn tpm_signature_scheme_clone() {
        let scheme = TpmSignatureScheme {
            scheme: 0x0018,
            hash_algorithm: Some(TpmNameAlgorithm::Sha256),
        };
        let cloned = scheme.clone();
        assert_eq!(scheme, cloned);
    }

    #[test]
    fn tpm_sign_command_params_clone() {
        let params = TpmSignCommandParams {
            key_handle: TpmHandle(0x8100_0001),
            digest: vec![0xAA; 32],
            scheme: TpmSignatureScheme {
                scheme: 0x0018,
                hash_algorithm: Some(TpmNameAlgorithm::Sha256),
            },
            validation_ticket_tag: TPM_ST_HASHCHECK,
            validation_ticket_hierarchy: TPM_RH_NULL,
            validation_digest: vec![],
        };
        let cloned = params.clone();
        assert_eq!(params, cloned);
    }

    #[test]
    fn tpm_hash_params_clone() {
        let params = TpmHashParams {
            data: b"hello".to_vec(),
            hash_algorithm: TpmNameAlgorithm::Sha256,
            hierarchy: TPM_RH_OWNER,
        };
        let cloned = params.clone();
        assert_eq!(params, cloned);
    }

    #[test]
    fn tpm_hash_check_ticket_clone() {
        let ticket = TpmHashCheckTicket {
            tag: TPM_ST_HASHCHECK,
            hierarchy: TPM_RH_NULL,
            digest: vec![0x01, 0x02],
        };
        let cloned = ticket.clone();
        assert_eq!(ticket, cloned);
    }

    #[test]
    fn tpm_hash_response_clone() {
        let resp = TpmHashResponse {
            digest: vec![0x01],
            validation: TpmHashCheckTicket {
                tag: TPM_ST_HASHCHECK,
                hierarchy: TPM_RH_NULL,
                digest: vec![],
            },
        };
        let cloned = resp.clone();
        assert_eq!(resp, cloned);
    }

    #[test]
    fn tpm_password_auth_session_clone() {
        let auth = TpmPasswordAuthSession {
            auth_value: vec![0x01, 0x02],
            session_attributes: 0x01,
        };
        let cloned = auth.clone();
        assert_eq!(auth, cloned);
    }

    #[test]
    fn tpm_start_auth_session_params_clone() {
        let params = TpmStartAuthSessionParams {
            tpm_key: TpmHandle(TPM_RH_NULL),
            bind: TpmHandle(TPM_RH_NULL),
            nonce_caller: vec![0x01],
            session_type: TpmSessionType::Policy,
            symmetric: TpmSymmetricDefinition::Null,
            auth_hash: TpmNameAlgorithm::Sha256,
        };
        let cloned = params.clone();
        assert_eq!(params, cloned);
    }

    #[test]
    fn tpm_policy_session_copy() {
        let session = TpmPolicySession {
            session_handle: 0x0300_0000,
            session_attributes: 0x01,
        };
        let s2 = session;
        let s3 = session;
        assert_eq!(s2, s3);
    }

    #[test]
    fn tpm_pcr_selection_clone() {
        let sel = TpmPcrSelection {
            hash_algorithm: TpmNameAlgorithm::Sha256,
            pcrs: vec![0, 7, 16],
        };
        let cloned = sel.clone();
        assert_eq!(sel, cloned);
    }

    #[test]
    fn tpm_policy_pcr_params_clone() {
        let params = TpmPolicyPcrParams {
            session: TpmPolicySession {
                session_handle: 0x0300_0000,
                session_attributes: 0,
            },
            pcr_digest: vec![0xAA; 32],
            selection: TpmPcrSelection {
                hash_algorithm: TpmNameAlgorithm::Sha256,
                pcrs: vec![0],
            },
        };
        let cloned = params.clone();
        assert_eq!(params, cloned);
    }

    #[test]
    fn tpm_policy_authorize_params_clone() {
        let params = TpmPolicyAuthorizeParams {
            session: TpmPolicySession {
                session_handle: 0x0300_0000,
                session_attributes: 0,
            },
            approved_policy: vec![0x01],
            policy_ref: vec![0x02],
            key_sign_name: vec![0x03],
            check_ticket_hierarchy: TPM_RH_NULL,
            check_ticket_digest: vec![0x04],
        };
        let cloned = params.clone();
        assert_eq!(params, cloned);
    }

    #[test]
    fn tpm_auth_command_clone() {
        let auth = TpmAuthCommand {
            session_handle: 0x0300_0000,
            nonce: vec![0x01, 0x02],
            session_attributes: 0x01,
            hmac: vec![0x03],
        };
        let cloned = auth.clone();
        assert_eq!(auth, cloned);
    }

    #[test]
    fn tpm_start_auth_session_response_clone() {
        let resp = TpmStartAuthSessionResponse {
            session_handle: 0x0300_0000,
            nonce_tpm: vec![0x01, 0x02],
        };
        let cloned = resp.clone();
        assert_eq!(resp, cloned);
    }

    #[test]
    fn tpm_parsed_signature_clone_all_variants() {
        let rsa = TpmParsedSignature::Rsa {
            scheme: 0x0014,
            hash_algorithm: TpmNameAlgorithm::Sha256,
            signature: vec![0x01],
        };
        assert_eq!(rsa.clone(), rsa);

        let ecc = TpmParsedSignature::Ecc {
            scheme: 0x0018,
            hash_algorithm: TpmNameAlgorithm::Sha384,
            r: vec![0x02],
            s: vec![0x03],
        };
        assert_eq!(ecc.clone(), ecc);

        let opaque = TpmParsedSignature::Opaque {
            scheme: 0x9999,
            bytes: vec![0x04],
        };
        assert_eq!(opaque.clone(), opaque);
    }

    #[test]
    fn tpm_parsed_signature_debug_format() {
        let sig = TpmParsedSignature::Rsa {
            scheme: 0x0014,
            hash_algorithm: TpmNameAlgorithm::Sha256,
            signature: vec![0x01],
        };
        let debug = format!("{:?}", sig);
        assert!(debug.contains("Rsa"));
    }

    #[test]
    fn encode_parsed_signature_rsa() {
        let sig = TpmParsedSignature::Rsa {
            scheme: 0x0014,
            hash_algorithm: TpmNameAlgorithm::Sha256,
            signature: vec![0xAA; 256],
        };
        let encoded = encode_parsed_signature(&sig);
        assert_eq!(&encoded[0..2], &0x0014u16.to_be_bytes());
        assert_eq!(&encoded[2..4], &0x000Bu16.to_be_bytes());
        assert_eq!(&encoded[4..6], &256u16.to_be_bytes());
        assert_eq!(&encoded[6..], &[0xAA; 256]);
    }

    #[test]
    fn encode_parsed_signature_ecc() {
        let sig = TpmParsedSignature::Ecc {
            scheme: 0x0018,
            hash_algorithm: TpmNameAlgorithm::Sha256,
            r: vec![0xBB; 32],
            s: vec![0xCC; 32],
        };
        let encoded = encode_parsed_signature(&sig);
        assert_eq!(&encoded[0..2], &0x0018u16.to_be_bytes());
        assert_eq!(&encoded[2..4], &0x000Bu16.to_be_bytes());
        assert_eq!(&encoded[4..6], &32u16.to_be_bytes());
        assert_eq!(&encoded[6..38], &[0xBB; 32]);
        assert_eq!(&encoded[38..40], &32u16.to_be_bytes());
        assert_eq!(&encoded[40..], &[0xCC; 32]);
    }

    #[test]
    fn encode_parsed_signature_opaque() {
        let sig = TpmParsedSignature::Opaque {
            scheme: 0xFFFF,
            bytes: vec![0xDD, 0xEE, 0xFF],
        };
        let encoded = encode_parsed_signature(&sig);
        assert_eq!(&encoded[0..2], &0xFFFFu16.to_be_bytes());
        assert_eq!(&encoded[2..], &[0xDD, 0xEE, 0xFF]);
    }

    #[test]
    fn encode_parsed_signature_empty_rsa() {
        let sig = TpmParsedSignature::Rsa {
            scheme: 0x0014,
            hash_algorithm: TpmNameAlgorithm::Null,
            signature: vec![],
        };
        let encoded = encode_parsed_signature(&sig);
        assert_eq!(&encoded[0..2], &0x0014u16.to_be_bytes());
        assert_eq!(&encoded[2..4], &0x0010u16.to_be_bytes());
        assert_eq!(&encoded[4..6], &0u16.to_be_bytes());
        assert_eq!(encoded.len(), 6);
    }

    #[test]
    fn build_auth_command_structure() {
        let auth = build_auth_command(&TpmAuthCommand {
            session_handle: 0x0300_0001,
            nonce: vec![0x11, 0x22],
            session_attributes: 0x03,
            hmac: vec![0x33, 0x44, 0x55],
        });
        assert_eq!(&auth[0..4], &0x0300_0001u32.to_be_bytes());
        assert_eq!(&auth[4..6], &2u16.to_be_bytes());
        assert_eq!(&auth[6..8], &[0x11, 0x22]);
        assert_eq!(auth[8], 0x03);
        assert_eq!(&auth[9..11], &3u16.to_be_bytes());
        assert_eq!(&auth[11..], &[0x33, 0x44, 0x55]);
    }

    #[test]
    fn build_auth_command_empty_fields() {
        let auth = build_auth_command(&TpmAuthCommand {
            session_handle: 0,
            nonce: vec![],
            session_attributes: 0,
            hmac: vec![],
        });
        assert_eq!(&auth[0..4], &0u32.to_be_bytes());
        assert_eq!(&auth[4..6], &0u16.to_be_bytes());
        assert_eq!(auth[6], 0);
        assert_eq!(&auth[7..9], &0u16.to_be_bytes());
        assert_eq!(auth.len(), 9);
    }

    #[test]
    fn build_password_auth_area_empty_auth_value() {
        let auth = TpmPasswordAuthSession {
            auth_value: vec![],
            session_attributes: 0,
        };
        let area = build_password_auth_area(&auth);
        assert_eq!(&area[0..4], &TPM_RS_PW.to_be_bytes());
        assert_eq!(&area[4..6], &0u16.to_be_bytes());
        assert_eq!(area[6], 0);
        assert_eq!(&area[7..9], &0u16.to_be_bytes());
        assert_eq!(area.len(), 9);
    }

    #[test]
    fn build_password_auth_area_with_auth_value() {
        let auth = TpmPasswordAuthSession {
            auth_value: vec![0xAA, 0xBB, 0xCC],
            session_attributes: 0x01,
        };
        let area = build_password_auth_area(&auth);
        assert_eq!(&area[0..4], &TPM_RS_PW.to_be_bytes());
        assert_eq!(&area[7..9], &3u16.to_be_bytes());
        assert_eq!(&area[9..], &[0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn default_sign_scheme_for_algorithm_all_variants() {
        assert_eq!(
            default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::RsaPkcs1v15Sha256).scheme,
            0x0014
        );
        assert_eq!(
            default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::RsaPssSha256).scheme,
            0x0016
        );
        assert_eq!(
            default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::EcdsaP256Sha256).scheme,
            0x0018
        );
        assert_eq!(
            default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::EcdsaP384Sha384).hash_algorithm,
            Some(TpmNameAlgorithm::Sha384)
        );
        assert_eq!(
            default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::EcSchnorr).scheme,
            0x001C
        );
        assert_eq!(
            default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::Eddsa).hash_algorithm,
            Some(TpmNameAlgorithm::Sha512)
        );
        let opaque_scheme = default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::Opaque("x".to_string()));
        assert_eq!(opaque_scheme.scheme, 0x0010);
        assert_eq!(opaque_scheme.hash_algorithm, None);
    }

    #[test]
    fn hash_message_for_algorithm_opaque_returns_error() {
        let err = hash_message_for_algorithm(&TpmSignatureAlgorithm::Opaque("bad".to_string()), b"msg").unwrap_err();
        assert!(matches!(err, TpmError::UnsupportedAlgorithm(_)));
    }

    #[test]
    fn sign_params_for_message_rsa() {
        let params = sign_params_for_message(
            TpmHandle(0x8100_0001),
            &TpmSignatureAlgorithm::RsaPkcs1v15Sha256,
            b"test message",
        ).unwrap();
        assert_eq!(params.key_handle, TpmHandle(0x8100_0001));
        assert_eq!(params.digest.len(), 32);
        assert_eq!(params.scheme.scheme, 0x0014);
        assert_eq!(params.validation_ticket_tag, TPM_ST_HASHCHECK);
        assert_eq!(params.validation_ticket_hierarchy, TPM_RH_NULL);
    }

    #[test]
    fn sign_params_for_message_ecdsa_p384() {
        let params = sign_params_for_message(
            TpmHandle(0x8100_0002),
            &TpmSignatureAlgorithm::EcdsaP384Sha384,
            b"hello",
        ).unwrap();
        assert_eq!(params.digest.len(), 48);
        assert_eq!(params.scheme.scheme, 0x0018);
        assert_eq!(params.scheme.hash_algorithm, Some(TpmNameAlgorithm::Sha384));
    }

    #[test]
    fn sign_params_for_message_eddsa_uses_sha512() {
        let params = sign_params_for_message(
            TpmHandle(0x8100_0003),
            &TpmSignatureAlgorithm::Eddsa,
            b"data",
        ).unwrap();
        assert_eq!(params.digest.len(), 64);
        assert_eq!(params.scheme.hash_algorithm, Some(TpmNameAlgorithm::Sha512));
    }

    #[test]
    fn sign_params_for_message_opaque_returns_error() {
        let err = sign_params_for_message(
            TpmHandle(0x8100_0001),
            &TpmSignatureAlgorithm::Opaque("bad".to_string()),
            b"data",
        ).unwrap_err();
        assert!(matches!(err, TpmError::UnsupportedAlgorithm(_)));
    }

    #[test]
    fn linux_tpm_device_new_and_path() {
        let device = LinuxTpmDevice::new("/dev/tpmrm1");
        assert_eq!(device.path(), Path::new("/dev/tpmrm1"));
    }

    #[test]
    fn linux_tpm_device_clone_and_debug() {
        let device = LinuxTpmDevice::new("/dev/tpm0");
        let cloned = device.clone();
        assert_eq!(device.path(), cloned.path());
        assert!(!format!("{:?}", device).is_empty());
    }

    #[test]
    fn linux_tpm_signing_key_defaults_to_no_auth() {
        let key = LinuxTpmSigningKey::new("/dev/tpmrm0", TpmHandle(0x8100_0001));
        assert_eq!(key.device_path(), Path::new("/dev/tpmrm0"));
        assert_eq!(key.handle(), TpmHandle(0x8100_0001));
        assert_eq!(key.authorization_mode(), &TpmAuthorizationMode::None);
    }

    #[test]
    fn linux_tpm_signing_key_with_auth_value() {
        let key = LinuxTpmSigningKey::new("/dev/tpmrm0", TpmHandle(0x8100_0001))
            .with_auth_value(vec![0x01, 0x02]);
        assert!(matches!(key.authorization_mode(), TpmAuthorizationMode::Password(_)));
    }

    #[test]
    fn linux_tpm_signing_key_with_password_auth_session() {
        let auth = TpmPasswordAuthSession {
            auth_value: vec![0xAA],
            session_attributes: 0x01,
        };
        let key = LinuxTpmSigningKey::new("/dev/tpmrm0", TpmHandle(0x8100_0001))
            .with_password_auth_session(auth.clone());
        assert_eq!(key.authorization_mode(), &TpmAuthorizationMode::Password(auth));
    }

    #[test]
    fn tpm_authorization_mode_clone_equality() {
        let m1 = TpmAuthorizationMode::None;
        let m2 = m1.clone();
        assert_eq!(m1, m2);

        let m3 = TpmAuthorizationMode::Password(TpmPasswordAuthSession {
            auth_value: vec![1],
            session_attributes: 0,
        });
        let m4 = m3.clone();
        assert_eq!(m3, m4);

        let m5 = TpmAuthorizationMode::Policy(TpmPolicySessionRunner::default());
        let m6 = m5.clone();
        assert_eq!(m5, m6);
    }

    #[test]
    fn tpm_device_transport_accessors() {
        let mut device = TpmDevice::new(FakePolicyTransport::new(vec![]));
        assert!(device.transport().commands.is_empty());
        assert!(device.transport_mut().commands.is_empty());
    }

    #[test]
    fn parse_read_public_response_public_area_raw_preserved() {
        let mut public = Vec::new();
        public.extend_from_slice(&0x0001u16.to_be_bytes()); // type = RSA
        public.extend_from_slice(&0x000Bu16.to_be_bytes()); // name_alg = SHA256
        public.extend_from_slice(&0x00030072u32.to_be_bytes()); // object_attributes
        public.extend_from_slice(&0u16.to_be_bytes()); // auth_policy length = 0
        public.extend_from_slice(&0x0010u16.to_be_bytes()); // sym = NULL
        public.extend_from_slice(&0x0010u16.to_be_bytes()); // scheme = NULL
        public.extend_from_slice(&2048u16.to_be_bytes()); // key_bits
        public.extend_from_slice(&0u32.to_be_bytes()); // exponent
        public.extend_from_slice(&0u16.to_be_bytes()); // unique length = 0

        let mut resp = Vec::new();
        let total = 10 + 2 + public.len() + 2 + 0 + 2 + 0;
        resp.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        resp.extend_from_slice(&(total as u32).to_be_bytes());
        resp.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        resp.extend_from_slice(&(public.len() as u16).to_be_bytes());
        resp.extend_from_slice(&public);
        resp.extend_from_slice(&0u16.to_be_bytes());
        resp.extend_from_slice(&0u16.to_be_bytes());

        let parsed = parse_read_public_response(&resp).unwrap();
        assert_eq!(parsed.public_area_raw, public);
    }

    #[test]
    fn key_info_from_read_public_rejects_keyed_hash() {
        let info = TpmReadPublicInfo {
            public_area: TpmPublicAreaInfo {
                object_type: TpmPublicObjectType::KeyedHash,
                name_algorithm: TpmNameAlgorithm::Sha256,
                object_attributes: 0,
                auth_policy: vec![],
                parameters: vec![],
                unique: vec![0x01],
                curve: None,
                key_bits: None,
            },
            public_area_raw: vec![],
            name: vec![],
            qualified_name: vec![],
        };
        let err = key_info_from_read_public(TpmHandle(0x8100_0001), &info).unwrap_err();
        assert_eq!(err, TpmError::UnsupportedPublicArea);
    }

    #[test]
    fn key_info_from_read_public_rejects_sym_cipher() {
        let info = TpmReadPublicInfo {
            public_area: TpmPublicAreaInfo {
                object_type: TpmPublicObjectType::SymCipher,
                name_algorithm: TpmNameAlgorithm::Sha256,
                object_attributes: 0,
                auth_policy: vec![],
                parameters: vec![],
                unique: vec![0x01],
                curve: None,
                key_bits: None,
            },
            public_area_raw: vec![],
            name: vec![],
            qualified_name: vec![],
        };
        let err = key_info_from_read_public(TpmHandle(0x8100_0001), &info).unwrap_err();
        assert_eq!(err, TpmError::UnsupportedPublicArea);
    }

    #[test]
    fn key_info_from_read_public_format_key_name() {
        let info = TpmReadPublicInfo {
            public_area: TpmPublicAreaInfo {
                object_type: TpmPublicObjectType::Ecc,
                name_algorithm: TpmNameAlgorithm::Sha256,
                object_attributes: 0,
                auth_policy: vec![],
                parameters: vec![],
                unique: vec![0xAA],
                curve: Some(TpmEccCurve::NistP256),
                key_bits: None,
            },
            public_area_raw: vec![],
            name: vec![],
            qualified_name: vec![],
        };
        let key = key_info_from_read_public(TpmHandle(0x0102_0304), &info).unwrap();
        assert_eq!(key.key_name, "tpm:0x01020304");
    }

    #[test]
    fn parse_public_area_rsa_with_non_null_scheme() {
        let mut public = Vec::new();
        public.extend_from_slice(&0x0001u16.to_be_bytes());
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        public.extend_from_slice(&0x00030072u32.to_be_bytes());
        public.extend_from_slice(&0u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        // non-NULL scheme with hash alg
        public.extend_from_slice(&0x0014u16.to_be_bytes());
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        public.extend_from_slice(&2048u16.to_be_bytes());
        public.extend_from_slice(&0u32.to_be_bytes());
        public.extend_from_slice(&0u16.to_be_bytes());

        let info = parse_public_area(&public).unwrap();
        assert_eq!(info.object_type, TpmPublicObjectType::Rsa);
        assert_eq!(info.key_bits, Some(2048));
    }

    #[test]
    fn parse_public_area_ecc_with_non_null_scheme() {
        let mut public = Vec::new();
        public.extend_from_slice(&0x0023u16.to_be_bytes()); // type = ECC
        public.extend_from_slice(&0x000Bu16.to_be_bytes()); // name_alg = SHA256
        public.extend_from_slice(&0x00030072u32.to_be_bytes()); // object_attributes
        public.extend_from_slice(&0u16.to_be_bytes()); // auth_policy len = 0
        public.extend_from_slice(&0x0010u16.to_be_bytes()); // sym_def = NULL
        // non-NULL ecc_scheme (0x0018) with hash_alg (0x000B)
        public.extend_from_slice(&0x0018u16.to_be_bytes()); // ecc_scheme
        public.extend_from_slice(&0x000Bu16.to_be_bytes()); // ecc_scheme_hash
        // curve
        public.extend_from_slice(&0x0003u16.to_be_bytes()); // curve = NistP256
        // NULL kdf
        public.extend_from_slice(&0x0010u16.to_be_bytes()); // kdf_scheme = NULL
        let x = vec![0x11; 32];
        let y = vec![0x22; 32];
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&x);
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&y);

        let info = parse_public_area(&public).unwrap();
        assert_eq!(info.object_type, TpmPublicObjectType::Ecc);
        assert_eq!(info.curve, Some(TpmEccCurve::NistP256));
    }

    #[test]
    fn parse_public_area_ecc_with_non_null_kdf() {
        let mut public = Vec::new();
        public.extend_from_slice(&0x0023u16.to_be_bytes());
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        public.extend_from_slice(&0x00030072u32.to_be_bytes());
        public.extend_from_slice(&0u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        public.extend_from_slice(&0x0003u16.to_be_bytes());
        // non-NULL kdf
        public.extend_from_slice(&0x0020u16.to_be_bytes());
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        let x = vec![0x11; 32];
        let y = vec![0x22; 32];
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&x);
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&y);

        let info = parse_public_area(&public).unwrap();
        assert_eq!(info.curve, Some(TpmEccCurve::NistP256));
    }

    #[test]
    fn parse_public_area_with_trailing_bytes_error() {
        let mut public = Vec::new();
        public.extend_from_slice(&0x0023u16.to_be_bytes());
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        public.extend_from_slice(&0x00030072u32.to_be_bytes());
        public.extend_from_slice(&0u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        public.extend_from_slice(&0x0003u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        let x = vec![0x11; 32];
        let y = vec![0x22; 32];
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&x);
        public.extend_from_slice(&32u16.to_be_bytes());
        public.extend_from_slice(&y);
        public.push(0xFF); // trailing

        let err = parse_public_area(&public).unwrap_err();
        assert!(err.to_string().contains("trailing bytes"));
    }

    #[test]
    fn parse_public_area_keyed_hash_type() {
        let mut public = Vec::new();
        public.extend_from_slice(&0x0008u16.to_be_bytes());
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        public.extend_from_slice(&0x00030072u32.to_be_bytes());
        public.extend_from_slice(&0u16.to_be_bytes());
        public.extend_from_slice(&[0xAA, 0xBB]);

        let info = parse_public_area(&public).unwrap();
        assert_eq!(info.object_type, TpmPublicObjectType::KeyedHash);
        assert_eq!(info.parameters, vec![0xAA, 0xBB]);
        assert!(info.unique.is_empty());
        assert_eq!(info.curve, None);
        assert_eq!(info.key_bits, None);
    }

    #[test]
    fn skip_sym_def_object_null_algorithm_skips_remaining() {
        let buf = [0x00, 0x10]; // NULL
        let mut cursor = 0usize;
        assert!(skip_sym_def_object(&buf, &mut cursor).is_ok());
        assert_eq!(cursor, 2);
    }

    #[test]
    fn skip_sym_def_object_non_null_consumes_all_fields() {
        let buf = [0x00, 0x11, 0x00, 0x80, 0x00, 0x40]; // alg=0x11, keybits=0x80, mode=0x40
        let mut cursor = 0usize;
        assert!(skip_sym_def_object(&buf, &mut cursor).is_ok());
        assert_eq!(cursor, 6);
    }

    #[test]
    fn skip_rsa_scheme_null_skips_remaining() {
        let buf = [0x00, 0x10]; // NULL
        let mut cursor = 0usize;
        assert!(skip_rsa_scheme(&buf, &mut cursor).is_ok());
        assert_eq!(cursor, 2);
    }

    #[test]
    fn skip_rsa_scheme_non_null_consumes_hash() {
        let buf = [0x00, 0x14, 0x00, 0x0B]; // scheme=0x14, hash=0x0B
        let mut cursor = 0usize;
        assert!(skip_rsa_scheme(&buf, &mut cursor).is_ok());
        assert_eq!(cursor, 4);
    }

    #[test]
    fn skip_ecc_scheme_null_skips_remaining() {
        let buf = [0x00, 0x10]; // NULL
        let mut cursor = 0usize;
        assert!(skip_ecc_scheme(&buf, &mut cursor).is_ok());
        assert_eq!(cursor, 2);
    }

    #[test]
    fn skip_kdf_scheme_null_skips_remaining() {
        let buf = [0x00, 0x10]; // NULL
        let mut cursor = 0usize;
        assert!(skip_kdf_scheme(&buf, &mut cursor).is_ok());
        assert_eq!(cursor, 2);
    }

    #[test]
    fn skip_kdf_scheme_non_null_consumes_hash() {
        let buf = [0x00, 0x20, 0x00, 0x0B]; // scheme=0x20, hash=0x0B
        let mut cursor = 0usize;
        assert!(skip_kdf_scheme(&buf, &mut cursor).is_ok());
        assert_eq!(cursor, 4);
    }

    #[test]
    fn constants_have_expected_values() {
        assert_eq!(TPM_ST_NO_SESSIONS, 0x8001);
        assert_eq!(TPM_ST_SESSIONS, 0x8002);
        assert_eq!(TPM_RC_SUCCESS, 0);
        assert_eq!(TPM_CC_READ_PUBLIC, 0x0000_0173);
        assert_eq!(TPM_CC_SIGN, 0x0000_015D);
        assert_eq!(TPM_CC_HASH, 0x0000_017D);
        assert_eq!(TPM_CC_VERIFY_SIGNATURE, 0x0000_0177);
        assert_eq!(TPM_CC_START_AUTH_SESSION, 0x0000_0176);
        assert_eq!(TPM_CC_POLICY_COMMAND_CODE, 0x0000_016C);
        assert_eq!(TPM_CC_POLICY_PCR, 0x0000_017F);
        assert_eq!(TPM_CC_POLICY_AUTHORIZE, 0x0000_016A);
        assert_eq!(TPM_RS_PW, 0x4000_0009);
        assert_eq!(TPM_RH_OWNER, 0x4000_0001);
        assert_eq!(TPM_RH_NULL, 0x4000_0007);
        assert_eq!(TPM_ST_HASHCHECK, 0x8024);
    }

    #[test]
    fn signature_input_for_record_delegates_to_core() {
        let input = signature_input_for_record("test:v0", &[0xAB; 32]);
        assert!(!input.is_empty());
    }

    #[test]
    fn tpm_policy_session_runner_default_values() {
        let runner = TpmPolicySessionRunner::default();
        assert_eq!(runner.nonce_caller.len(), 16);
        assert!(runner.nonce_caller.iter().all(|&b| b == 0));
        assert_eq!(runner.auth_hash, TpmNameAlgorithm::Sha256);
        assert_eq!(runner.session_attributes, 0);
        assert!(runner.pcr_policy.is_none());
        assert!(runner.pcr_digest.is_empty());
        assert!(runner.authorize_policy.is_none());
    }

    #[test]
    fn tpm_policy_session_runner_with_pcr_policy() {
        let sel = TpmPcrSelection {
            hash_algorithm: TpmNameAlgorithm::Sha256,
            pcrs: vec![0, 1],
        };
        let digest = vec![0xAA; 32];
        let runner = TpmPolicySessionRunner::default().with_pcr_policy(sel.clone(), digest.clone());
        assert_eq!(runner.pcr_policy, Some(sel));
        assert_eq!(runner.pcr_digest, digest);
    }

    #[test]
    fn tpm_policy_session_runner_with_policy_authorize() {
        let runner = TpmPolicySessionRunner::default().with_policy_authorize(
            vec![0x01],
            vec![0x02],
            vec![0x03],
            TPM_RH_NULL,
            vec![0x04],
        );
        let auth = runner.authorize_policy.unwrap();
        assert_eq!(auth.approved_policy, vec![0x01]);
        assert_eq!(auth.policy_ref, vec![0x02]);
        assert_eq!(auth.key_sign_name, vec![0x03]);
        assert_eq!(auth.check_ticket_hierarchy, TPM_RH_NULL);
        assert_eq!(auth.check_ticket_digest, vec![0x04]);
    }

    #[test]
    fn build_sign_command_with_policy_session_size_matches_actual() {
        let cmd = build_sign_command_with_policy_session(
            &TpmSignCommandParams {
                key_handle: TpmHandle(0x8100_0001),
                digest: vec![0xAA; 32],
                scheme: default_sign_scheme_for_algorithm(&TpmSignatureAlgorithm::EcdsaP256Sha256),
                validation_ticket_tag: TPM_ST_HASHCHECK,
                validation_ticket_hierarchy: TPM_RH_NULL,
                validation_digest: vec![0xBB; 4],
            },
            TpmPolicySession {
                session_handle: 0x0300_0000,
                session_attributes: 0x01,
            },
        );
        let declared_size = u32::from_be_bytes(cmd[2..6].try_into().unwrap());
        assert_eq!(declared_size as usize, cmd.len());
    }

    #[test]
    fn build_policy_pcr_command_size_matches_actual() {
        let cmd = build_policy_pcr_command(&TpmPolicyPcrParams {
            session: TpmPolicySession {
                session_handle: 0x0300_0000,
                session_attributes: 0,
            },
            pcr_digest: vec![0xAA; 32],
            selection: TpmPcrSelection {
                hash_algorithm: TpmNameAlgorithm::Sha256,
                pcrs: vec![0, 7, 15],
            },
        });
        let declared_size = u32::from_be_bytes(cmd[2..6].try_into().unwrap());
        assert_eq!(declared_size as usize, cmd.len());
    }

    #[test]
    fn build_policy_authorize_command_size_matches_actual() {
        let cmd = build_policy_authorize_command(&TpmPolicyAuthorizeParams {
            session: TpmPolicySession {
                session_handle: 0x0300_0000,
                session_attributes: 0,
            },
            approved_policy: vec![0x11; 32],
            policy_ref: vec![0x22; 16],
            key_sign_name: vec![0x33; 8],
            check_ticket_hierarchy: TPM_RH_OWNER,
            check_ticket_digest: vec![0x44; 20],
        });
        let declared_size = u32::from_be_bytes(cmd[2..6].try_into().unwrap());
        assert_eq!(declared_size as usize, cmd.len());
    }

    #[test]
    fn build_start_auth_session_command_size_matches_actual() {
        let cmd = build_start_auth_session_command(&TpmStartAuthSessionParams {
            tpm_key: TpmHandle(0x8100_0001),
            bind: TpmHandle(0x4000_0001),
            nonce_caller: vec![0xCC; 20],
            session_type: TpmSessionType::Hmac,
            symmetric: TpmSymmetricDefinition::Null,
            auth_hash: TpmNameAlgorithm::Sha1,
        });
        let declared_size = u32::from_be_bytes(cmd[2..6].try_into().unwrap());
        assert_eq!(declared_size as usize, cmd.len());
    }

    #[test]
    fn build_verify_signature_command_size_matches_actual() {
        let cmd = build_verify_signature_command(
            TpmHandle(0x8100_0001),
            &[0xDD; 32],
            &TpmParsedSignature::Rsa {
                scheme: 0x0014,
                hash_algorithm: TpmNameAlgorithm::Sha256,
                signature: vec![0xEE; 128],
            },
        );
        let declared_size = u32::from_be_bytes(cmd[2..6].try_into().unwrap());
        assert_eq!(declared_size as usize, cmd.len());
    }

    #[test]
    fn read_public_response_with_large_name() {
        let mut public = Vec::new();
        public.extend_from_slice(&0x0001u16.to_be_bytes());
        public.extend_from_slice(&0x000Bu16.to_be_bytes());
        public.extend_from_slice(&0x00030072u32.to_be_bytes());
        public.extend_from_slice(&0u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        public.extend_from_slice(&0x0010u16.to_be_bytes());
        public.extend_from_slice(&2048u16.to_be_bytes());
        public.extend_from_slice(&0u32.to_be_bytes());
        public.extend_from_slice(&0u16.to_be_bytes());

        let name = vec![0xFF; 200];
        let qname = vec![0xEE; 100];

        let mut resp = Vec::new();
        let total = 10 + 2 + public.len() + 2 + name.len() + 2 + qname.len();
        resp.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        resp.extend_from_slice(&(total as u32).to_be_bytes());
        resp.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        resp.extend_from_slice(&(public.len() as u16).to_be_bytes());
        resp.extend_from_slice(&public);
        resp.extend_from_slice(&(name.len() as u16).to_be_bytes());
        resp.extend_from_slice(&name);
        resp.extend_from_slice(&(qname.len() as u16).to_be_bytes());
        resp.extend_from_slice(&qname);

        let parsed = parse_read_public_response(&resp).unwrap();
        assert_eq!(parsed.name, name);
        assert_eq!(parsed.qualified_name, qname);
    }

    #[test]
    fn sign_prehashed_with_device_uses_none_mode() {
        let mut body = Vec::new();
        body.extend_from_slice(&0x0018u16.to_be_bytes());
        body.extend_from_slice(&0x000Bu16.to_be_bytes());
        body.extend_from_slice(&2u16.to_be_bytes());
        body.extend_from_slice(&[1, 2]);
        body.extend_from_slice(&2u16.to_be_bytes());
        body.extend_from_slice(&[3, 4]);
        let total = 10 + body.len();
        let mut sign_response = Vec::new();
        sign_response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        sign_response.extend_from_slice(&(total as u32).to_be_bytes());
        sign_response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        sign_response.extend_from_slice(&body);

        let mut device = TpmDevice::new(FakePolicyTransport::new(vec![sign_response]));
        let parsed = sign_prehashed_with_device(
            &mut device,
            &TpmAuthorizationMode::None,
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
        assert!(matches!(parsed, TpmParsedSignature::Ecc { .. }));
        assert_eq!(device.transport().commands.len(), 1);
        // None mode now uses an empty password auth session (TPM_ST_SESSIONS)
        // because persisted keys with userWithAuth require at least an empty password session
        assert_eq!(&device.transport().commands[0][0..2], &TPM_ST_SESSIONS.to_be_bytes());
    }

    #[test]
    fn parse_sign_response_ecc_with_sha384() {
        let mut response = Vec::new();
        let mut body = Vec::new();
        body.extend_from_slice(&0x0018u16.to_be_bytes());
        body.extend_from_slice(&0x000Cu16.to_be_bytes());
        body.extend_from_slice(&3u16.to_be_bytes());
        body.extend_from_slice(&[1, 2, 3]);
        body.extend_from_slice(&3u16.to_be_bytes());
        body.extend_from_slice(&[4, 5, 6]);
        response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response.extend_from_slice(&((10 + body.len()) as u32).to_be_bytes());
        response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
        response.extend_from_slice(&body);

        let parsed = parse_sign_response(&response).unwrap();
        assert_eq!(
            parsed,
            TpmParsedSignature::Ecc {
                scheme: 0x0018,
                hash_algorithm: TpmNameAlgorithm::Sha384,
                r: vec![1, 2, 3],
                s: vec![4, 5, 6],
            }
        );
    }

    #[test]
    fn parse_sign_response_rsa_with_sha1() {
        let mut response = Vec::new();
        let mut body = Vec::new();
        body.extend_from_slice(&0x0014u16.to_be_bytes());
        body.extend_from_slice(&0x0004u16.to_be_bytes());
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
                scheme: 0x0014,
                hash_algorithm: TpmNameAlgorithm::Sha1,
                signature: vec![9, 8, 7, 6],
            }
        );
    }

    #[test]
    fn hash_message_for_algorithm_sha256_variants() {
        let algos = [
            TpmSignatureAlgorithm::RsaPkcs1v15Sha256,
            TpmSignatureAlgorithm::RsaPssSha256,
            TpmSignatureAlgorithm::EcdsaP256Sha256,
            TpmSignatureAlgorithm::EcSchnorr,
        ];
        for algo in algos {
            let digest = hash_message_for_algorithm(&algo, b"test").unwrap();
            assert_eq!(digest.len(), 32);
        }
    }
}
