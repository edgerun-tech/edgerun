extern crate alloc;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use edgerun_crypto::sealing::{SealedBytes, seal_aes256_gcm, unseal_aes256_gcm};
use edgerun_crypto::{fill_random, sha256};

use crate::record_codec::{UiRecordError, UiRecordReader, UiRecordWriter};

pub const PASSWORD_MIN_LEN: usize = 8;
pub const PASSWORD_SALT_LEN: usize = 32;
pub const LOCAL_SECRET_LEN: usize = 32;
pub const DEFAULT_KDF_ROUNDS: u32 = 120_000;

const KDF_DOMAIN: &[u8] = b"edgerun:v1:ui:initial-setup:password-root:kdf";
const VERIFIER_DOMAIN: &[u8] = b"edgerun:v1:ui:initial-setup:password-root:verifier";
const SECRET_SEAL_AAD: &[u8] = b"edgerun:v1:ui:initial-setup:local-secret";
const DEVICE_GRANT_DOMAIN: &[u8] = b"edgerun:v1:ui:initial-setup:user-device-admission-grant";
const FINGERPRINT_RECORD_MAGIC: &[u8; 5] = b"ERFG1";
const TPM_RECORD_MAGIC: &[u8; 5] = b"ERTD1";
const YUBIKEY_RECORD_MAGIC: &[u8; 5] = b"ERYU1";
const DEVICE_GRANT_RECORD_MAGIC: &[u8; 5] = b"ERDG1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InitialSetupError {
    PasswordTooShort,
    RandomFailed,
    SealFailed,
    UnsealFailed,
    InvalidRecord,
    PasswordRejected,
    Io(String),
}

impl From<UiRecordError> for InitialSetupError {
    fn from(_value: UiRecordError) -> Self {
        Self::InvalidRecord
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasswordRootRecord {
    pub salt: [u8; PASSWORD_SALT_LEN],
    pub verifier: [u8; LOCAL_SECRET_LEN],
    pub secret_envelope: Vec<u8>,
    pub kdf_rounds: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasswordRootSetup {
    pub record: PasswordRootRecord,
    pub local_secret: [u8; LOCAL_SECRET_LEN],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PasswordRootStatus {
    pub verifier: [u8; LOCAL_SECRET_LEN],
    pub envelope_len: usize,
    pub kdf_rounds: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupUnlockMode {
    PasswordRoot,
    PasswordlessHardware,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PasswordlessAuthorityState {
    pub fingerprint_presence: Option<FingerprintPresenceRef>,
    pub tpm_device_authority: Option<TpmDeviceAuthorityRef>,
    pub yubikey_user_authority: Option<YubiKeyUserAuthorityRef>,
    pub user_device_grant: Option<UserDeviceAdmissionGrant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FingerprintPresenceRef {
    pub provider: String,
    pub template_ref: String,
}

impl FingerprintPresenceRef {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = UiRecordWriter::new(FINGERPRINT_RECORD_MAGIC);
        writer
            .string(&self.provider)
            .and_then(|writer| writer.string(&self.template_ref))
            .expect("fingerprint presence record length exceeds u32");
        writer.into_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InitialSetupError> {
        let mut cursor = UiRecordReader::new(bytes, FINGERPRINT_RECORD_MAGIC)?;
        Ok(Self {
            provider: cursor.read_string()?,
            template_ref: cursor.read_string()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmDeviceAuthorityRef {
    pub key_name: String,
    pub public_key: Vec<u8>,
}

impl TpmDeviceAuthorityRef {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = UiRecordWriter::new(TPM_RECORD_MAGIC);
        writer
            .string(&self.key_name)
            .and_then(|writer| writer.bytes(&self.public_key))
            .expect("TPM authority record length exceeds u32");
        writer.into_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InitialSetupError> {
        let mut cursor = UiRecordReader::new(bytes, TPM_RECORD_MAGIC)?;
        Ok(Self {
            key_name: cursor.read_string()?,
            public_key: cursor.read_bytes()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YubiKeyUserAuthorityRef {
    pub slot: String,
    pub public_key: Vec<u8>,
    pub attestation_chain: Vec<Vec<u8>>,
}

impl YubiKeyUserAuthorityRef {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = UiRecordWriter::new(YUBIKEY_RECORD_MAGIC);
        writer
            .string(&self.slot)
            .and_then(|writer| writer.bytes(&self.public_key))
            .and_then(|writer| writer.bytes_vec(&self.attestation_chain))
            .expect("YubiKey authority record length exceeds u32");
        writer.into_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InitialSetupError> {
        let mut cursor = UiRecordReader::new(bytes, YUBIKEY_RECORD_MAGIC)?;
        Ok(Self {
            slot: cursor.read_string()?,
            public_key: cursor.read_bytes()?,
            attestation_chain: cursor.read_bytes_vec()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserDeviceAdmissionGrant {
    pub user_public_key: Vec<u8>,
    pub user_key_label: String,
    pub device_public_key: Vec<u8>,
    pub device_key_label: String,
    pub fingerprint_template_ref: String,
    pub allowed_roles: Vec<String>,
    pub allowed_hardware_scopes: Vec<String>,
    pub presence_required: bool,
    pub policy_hash: [u8; 32],
    pub issued_at_secs: u64,
    pub expires_at_secs: u64,
    pub user_signature: Vec<u8>,
}

impl UserDeviceAdmissionGrant {
    pub fn signing_preimage(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_bytes(&mut out, DEVICE_GRANT_DOMAIN);
        push_bytes(&mut out, &self.user_public_key);
        push_string(&mut out, &self.user_key_label);
        push_bytes(&mut out, &self.device_public_key);
        push_string(&mut out, &self.device_key_label);
        push_string(&mut out, &self.fingerprint_template_ref);
        push_string_vec(&mut out, &self.allowed_roles);
        push_string_vec(&mut out, &self.allowed_hardware_scopes);
        out.push(u8::from(self.presence_required));
        out.extend_from_slice(&self.policy_hash);
        out.extend_from_slice(&self.issued_at_secs.to_le_bytes());
        out.extend_from_slice(&self.expires_at_secs.to_le_bytes());
        out
    }

    pub fn claim_hash(&self) -> [u8; 32] {
        sha256(&self.signing_preimage())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = UiRecordWriter::new(DEVICE_GRANT_RECORD_MAGIC);
        writer
            .bytes(&self.user_public_key)
            .and_then(|writer| writer.string(&self.user_key_label))
            .and_then(|writer| writer.bytes(&self.device_public_key))
            .and_then(|writer| writer.string(&self.device_key_label))
            .and_then(|writer| writer.string(&self.fingerprint_template_ref))
            .and_then(|writer| writer.string_vec(&self.allowed_roles))
            .and_then(|writer| writer.string_vec(&self.allowed_hardware_scopes))
            .expect("device grant record length exceeds u32")
            .bool(self.presence_required)
            .array_32(&self.policy_hash)
            .u64(self.issued_at_secs)
            .u64(self.expires_at_secs)
            .bytes(&self.user_signature)
            .expect("device grant signature length exceeds u32");
        writer.into_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InitialSetupError> {
        let mut cursor = UiRecordReader::new(bytes, DEVICE_GRANT_RECORD_MAGIC)?;
        let user_public_key = cursor.read_bytes()?;
        let user_key_label = cursor.read_string()?;
        let device_public_key = cursor.read_bytes()?;
        let device_key_label = cursor.read_string()?;
        let fingerprint_template_ref = cursor.read_string()?;
        let allowed_roles = cursor.read_string_vec()?;
        let allowed_hardware_scopes = cursor.read_string_vec()?;
        let presence_required = cursor.read_bool()?;
        let policy_hash = cursor.read_array_32()?;
        let issued_at_secs = cursor.read_u64()?;
        let expires_at_secs = cursor.read_u64()?;
        let user_signature = cursor.read_bytes()?;
        cursor.finish()?;
        Ok(Self {
            user_public_key,
            user_key_label,
            device_public_key,
            device_key_label,
            fingerprint_template_ref,
            allowed_roles,
            allowed_hardware_scopes,
            presence_required,
            policy_hash,
            issued_at_secs,
            expires_at_secs,
            user_signature,
        })
    }

    pub fn from_unsigned_parts(
        user: &YubiKeyUserAuthorityRef,
        device: &TpmDeviceAuthorityRef,
        fingerprint: &FingerprintPresenceRef,
        issued_at_secs: u64,
        expires_at_secs: u64,
    ) -> Self {
        let allowed_roles = vec![
            "admission:device".into(),
            "notary:device".into(),
            "storage:local-cache".into(),
        ];
        let allowed_hardware_scopes = vec![
            "tpm:sign".into(),
            "fingerprint:presence".into(),
            "systemd:staged-apply".into(),
        ];
        let policy_hash = user_device_policy_hash(
            &user.public_key,
            &device.public_key,
            &fingerprint.template_ref,
            &allowed_roles,
            &allowed_hardware_scopes,
        );
        Self {
            user_public_key: user.public_key.clone(),
            user_key_label: user.slot.clone(),
            device_public_key: device.public_key.clone(),
            device_key_label: device.key_name.clone(),
            fingerprint_template_ref: fingerprint.template_ref.clone(),
            allowed_roles,
            allowed_hardware_scopes,
            presence_required: true,
            policy_hash,
            issued_at_secs,
            expires_at_secs,
            user_signature: Vec::new(),
        }
    }
}

pub fn user_device_policy_hash(
    user_public_key: &[u8],
    device_public_key: &[u8],
    fingerprint_template_ref: &str,
    allowed_roles: &[String],
    allowed_hardware_scopes: &[String],
) -> [u8; 32] {
    let mut out = Vec::new();
    push_bytes(&mut out, b"edgerun:v1:ui:initial-setup:user-device-policy");
    push_bytes(&mut out, user_public_key);
    push_bytes(&mut out, device_public_key);
    push_string(&mut out, fingerprint_template_ref);
    push_string_vec(&mut out, allowed_roles);
    push_string_vec(&mut out, allowed_hardware_scopes);
    sha256(&out)
}

pub fn create_password_root(password: &str) -> Result<PasswordRootSetup, InitialSetupError> {
    create_password_root_with_rounds(password, DEFAULT_KDF_ROUNDS)
}

pub fn create_password_root_with_rounds(
    password: &str,
    kdf_rounds: u32,
) -> Result<PasswordRootSetup, InitialSetupError> {
    validate_password(password)?;
    let mut salt = [0u8; PASSWORD_SALT_LEN];
    fill_random(&mut salt).map_err(|_| InitialSetupError::RandomFailed)?;
    let wrapping_key = derive_password_key(password.as_bytes(), &salt, kdf_rounds);

    let mut local_secret = [0u8; LOCAL_SECRET_LEN];
    fill_random(&mut local_secret).map_err(|_| InitialSetupError::RandomFailed)?;
    let sealed = seal_aes256_gcm(&wrapping_key, SECRET_SEAL_AAD, &local_secret)
        .map_err(|_| InitialSetupError::SealFailed)?;

    Ok(PasswordRootSetup {
        record: PasswordRootRecord {
            salt,
            verifier: password_verifier(&wrapping_key),
            secret_envelope: sealed.to_bytes(),
            kdf_rounds,
        },
        local_secret,
    })
}

pub fn unlock_password_root(
    password: &str,
    record: &PasswordRootRecord,
) -> Result<[u8; LOCAL_SECRET_LEN], InitialSetupError> {
    if record.kdf_rounds == 0
        || record.secret_envelope.is_empty()
        || password.len() < PASSWORD_MIN_LEN
    {
        return Err(InitialSetupError::InvalidRecord);
    }
    let wrapping_key = derive_password_key(password.as_bytes(), &record.salt, record.kdf_rounds);
    if password_verifier(&wrapping_key) != record.verifier {
        return Err(InitialSetupError::PasswordRejected);
    }
    let sealed = SealedBytes::from_bytes(&record.secret_envelope)
        .map_err(|_| InitialSetupError::InvalidRecord)?;
    let plaintext = unseal_aes256_gcm(&wrapping_key, SECRET_SEAL_AAD, &sealed)
        .map_err(|_| InitialSetupError::UnsealFailed)?;
    if plaintext.len() != LOCAL_SECRET_LEN {
        return Err(InitialSetupError::InvalidRecord);
    }
    let mut secret = [0u8; LOCAL_SECRET_LEN];
    secret.copy_from_slice(&plaintext);
    Ok(secret)
}

pub fn password_root_status(record: &PasswordRootRecord) -> PasswordRootStatus {
    PasswordRootStatus {
        verifier: record.verifier,
        envelope_len: record.secret_envelope.len(),
        kdf_rounds: record.kdf_rounds,
    }
}

fn validate_password(password: &str) -> Result<(), InitialSetupError> {
    if password.len() < PASSWORD_MIN_LEN {
        return Err(InitialSetupError::PasswordTooShort);
    }
    Ok(())
}

fn derive_password_key(password: &[u8], salt: &[u8; PASSWORD_SALT_LEN], rounds: u32) -> [u8; 32] {
    let rounds = rounds.max(1);
    let mut state = sha256_join(&[KDF_DOMAIN, b"init", salt, password]);
    let mut counter = 0u32;
    while counter < rounds {
        let counter_bytes = counter.to_le_bytes();
        state = sha256_join(&[KDF_DOMAIN, b"round", salt, &counter_bytes, &state, password]);
        counter += 1;
    }
    state
}

fn password_verifier(wrapping_key: &[u8; 32]) -> [u8; 32] {
    sha256_join(&[VERIFIER_DOMAIN, wrapping_key])
}

fn sha256_join(parts: &[&[u8]]) -> [u8; 32] {
    let len = parts.iter().map(|part| part.len()).sum();
    let mut bytes = Vec::with_capacity(len);
    for part in parts {
        bytes.extend_from_slice(part);
    }
    sha256(&bytes)
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    push_bytes(out, value.as_bytes());
}

fn push_string_vec(out: &mut Vec<u8>, values: &[String]) {
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for value in values {
        push_string(out, value);
    }
}

#[cfg(feature = "std")]
pub mod file_store {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    pub const PASSWORD_SALT_FILE: &str = "password-root.salt";
    pub const PASSWORD_VERIFIER_FILE: &str = "password-root.verifier";
    pub const PASSWORD_SECRET_FILE: &str = "password-root.envelope";
    pub const PASSWORD_KDF_ROUNDS_FILE: &str = "password-root.kdf-rounds";

    #[derive(Clone, Debug)]
    pub struct PasswordRootFileStore {
        root: PathBuf,
    }

    impl PasswordRootFileStore {
        pub fn new(root: PathBuf) -> Self {
            Self { root }
        }

        pub fn root(&self) -> &Path {
            &self.root
        }

        pub fn create(&self, password: &str) -> Result<PasswordRootStatus, InitialSetupError> {
            let setup = create_password_root(password)?;
            self.persist(&setup.record)?;
            Ok(password_root_status(&setup.record))
        }

        pub fn load(&self) -> Result<PasswordRootRecord, InitialSetupError> {
            let salt_bytes = fs::read(self.root.join(PASSWORD_SALT_FILE)).map_err(|err| {
                InitialSetupError::Io(format!("read password salt failed: {err}"))
            })?;
            let verifier_bytes =
                fs::read(self.root.join(PASSWORD_VERIFIER_FILE)).map_err(|err| {
                    InitialSetupError::Io(format!("read password verifier failed: {err}"))
                })?;
            let secret_envelope =
                fs::read(self.root.join(PASSWORD_SECRET_FILE)).map_err(|err| {
                    InitialSetupError::Io(format!("read password secret envelope failed: {err}"))
                })?;
            let kdf_rounds = fs::read_to_string(self.root.join(PASSWORD_KDF_ROUNDS_FILE))
                .ok()
                .and_then(|value| value.trim().parse::<u32>().ok())
                .unwrap_or(DEFAULT_KDF_ROUNDS);
            if salt_bytes.len() != PASSWORD_SALT_LEN
                || verifier_bytes.len() != LOCAL_SECRET_LEN
                || secret_envelope.is_empty()
                || kdf_rounds == 0
            {
                return Err(InitialSetupError::InvalidRecord);
            }
            let mut salt = [0u8; PASSWORD_SALT_LEN];
            salt.copy_from_slice(&salt_bytes);
            let mut verifier = [0u8; LOCAL_SECRET_LEN];
            verifier.copy_from_slice(&verifier_bytes);
            Ok(PasswordRootRecord {
                salt,
                verifier,
                secret_envelope,
                kdf_rounds,
            })
        }

        pub fn status(&self) -> Result<PasswordRootStatus, InitialSetupError> {
            let record = self.load()?;
            Ok(password_root_status(&record))
        }

        pub fn unlock(&self, password: &str) -> Result<[u8; LOCAL_SECRET_LEN], InitialSetupError> {
            let record = self.load()?;
            unlock_password_root(password, &record)
        }

        pub fn persist(&self, record: &PasswordRootRecord) -> Result<(), InitialSetupError> {
            fs::create_dir_all(&self.root).map_err(|err| {
                InitialSetupError::Io(format!(
                    "create setup root {} failed: {err}",
                    self.root.display()
                ))
            })?;
            fs::write(self.root.join(PASSWORD_SALT_FILE), record.salt).map_err(|err| {
                InitialSetupError::Io(format!("write password salt failed: {err}"))
            })?;
            fs::write(self.root.join(PASSWORD_VERIFIER_FILE), record.verifier).map_err(|err| {
                InitialSetupError::Io(format!("write password verifier failed: {err}"))
            })?;
            fs::write(
                self.root.join(PASSWORD_SECRET_FILE),
                &record.secret_envelope,
            )
            .map_err(|err| {
                InitialSetupError::Io(format!("write password secret envelope failed: {err}"))
            })?;
            fs::write(
                self.root.join(PASSWORD_KDF_ROUNDS_FILE),
                record.kdf_rounds.to_string().as_bytes(),
            )
            .map_err(|err| InitialSetupError::Io(format!("write KDF rounds failed: {err}")))?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn password_root_round_trips_secret_without_storing_password() {
        let setup = create_password_root_with_rounds("correct horse battery", 8)
            .expect("password root setup");

        let unlocked = unlock_password_root("correct horse battery", &setup.record)
            .expect("password root unlock");
        assert_eq!(unlocked, setup.local_secret);
        assert_eq!(
            unlock_password_root("wrong horse battery", &setup.record),
            Err(InitialSetupError::PasswordRejected)
        );
        assert!(
            !setup
                .record
                .secret_envelope
                .windows("correct horse battery".len())
                .any(|window| window == b"correct horse battery")
        );
    }

    #[test]
    fn user_device_grant_hash_changes_with_authority_material() {
        let user = YubiKeyUserAuthorityRef {
            slot: "9c".to_string(),
            public_key: vec![1; 65],
            attestation_chain: vec![vec![9; 4]],
        };
        let device = TpmDeviceAuthorityRef {
            key_name: "tpm:0x81000001".to_string(),
            public_key: vec![2; 64],
        };
        let fingerprint = FingerprintPresenceRef {
            provider: "goodix".to_string(),
            template_ref: "template-a".to_string(),
        };
        let mut grant =
            UserDeviceAdmissionGrant::from_unsigned_parts(&user, &device, &fingerprint, 10, 20);
        let claim_hash = grant.claim_hash();
        grant.user_signature = vec![3; 64];

        assert_eq!(claim_hash, grant.claim_hash());
        assert!(!grant.signing_preimage().windows(64).any(|w| w == &[3; 64]));
        assert!(!grant.to_bytes().is_empty());

        let changed_fingerprint = FingerprintPresenceRef {
            provider: "goodix".to_string(),
            template_ref: "template-b".to_string(),
        };
        let changed = UserDeviceAdmissionGrant::from_unsigned_parts(
            &user,
            &device,
            &changed_fingerprint,
            10,
            20,
        );
        assert_ne!(claim_hash, changed.claim_hash());
    }

    #[test]
    fn authority_records_round_trip_without_json() {
        let user = YubiKeyUserAuthorityRef {
            slot: "9c".to_string(),
            public_key: vec![1; 65],
            attestation_chain: vec![vec![9; 4], vec![8; 3]],
        };
        let device = TpmDeviceAuthorityRef {
            key_name: "tpm:0x81000001".to_string(),
            public_key: vec![2; 64],
        };
        let fingerprint = FingerprintPresenceRef {
            provider: "goodix".to_string(),
            template_ref: "template-a".to_string(),
        };
        let mut grant =
            UserDeviceAdmissionGrant::from_unsigned_parts(&user, &device, &fingerprint, 10, 20);
        grant.user_signature = vec![3; 64];

        assert_eq!(
            FingerprintPresenceRef::from_bytes(&fingerprint.to_bytes()).unwrap(),
            fingerprint
        );
        assert_eq!(
            TpmDeviceAuthorityRef::from_bytes(&device.to_bytes()).unwrap(),
            device
        );
        assert_eq!(
            YubiKeyUserAuthorityRef::from_bytes(&user.to_bytes()).unwrap(),
            user
        );
        assert_eq!(
            UserDeviceAdmissionGrant::from_bytes(&grant.to_bytes()).unwrap(),
            grant
        );
        assert_eq!(
            UserDeviceAdmissionGrant::from_bytes(&grant.signing_preimage()),
            Err(InitialSetupError::InvalidRecord)
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn file_store_persists_record_and_unlocks_secret() {
        let root = std::env::temp_dir().join(format!(
            "edgerun-ui-password-root-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = file_store::PasswordRootFileStore::new(root.clone());
        let setup = create_password_root_with_rounds("correct horse battery", 8)
            .expect("password root setup");

        store.persist(&setup.record).expect("persist record");
        assert_eq!(
            store.unlock("correct horse battery").expect("unlock"),
            setup.local_secret
        );
        assert_eq!(
            store.unlock("wrong horse battery"),
            Err(InitialSetupError::PasswordRejected)
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
