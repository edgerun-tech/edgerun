use std::cell::RefCell;
use std::format;
use std::fs;
use std::path::{Path, PathBuf};
use std::string::{String, ToString};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec;
use std::vec::Vec;

use edgerun_crypto::{fill_random, sha256};
use edgerun_protocols::keygen::{generate_node_signing_key, node_id_from_signing_key};
use edgerun_protocols::seal::{
    SealKey, generate_seal_key, seal_node_signing_key, seal_with_key, unseal_node_signing_key,
    unseal_with_key,
};
use edgerun_ui_core::gpu::sdl::{
    SDL_KEY_ESCAPE, SdlEventResult, SdlGlWindowOptions, SdlInputEvent, run_sdl_gl_window,
};
use edgerun_ui_core::gpu::{ButtonStyle, Color4, HitKind, UiAction, UiIcon, UiNode};

use crate::capacity::{NodeCapacity, format_bytes};
use crate::hardware::HardwareInventory;

const SDLK_R: i32 = 114;

const BG: Color4 = Color4::rgba(0.030, 0.034, 0.041, 1.0);
const ACCENT: Color4 = Color4::rgba(0.090, 0.650, 0.530, 1.0);
const BLUE: Color4 = Color4::rgba(0.280, 0.560, 0.920, 1.0);
const AMBER: Color4 = Color4::rgba(0.920, 0.660, 0.250, 1.0);
const NAV_MACHINE_REPORT_ID: u32 = 10_001;
const NAV_NODE_INSTANCES_ID: u32 = 10_002;
const NAV_STORAGE_ID: u32 = 10_003;
const NAV_TRUST_ID: u32 = 10_004;
const NAV_SERVICES_ID: u32 = 10_005;
const ACTION_STORAGE_LOCAL_CACHE: u32 = 20_101;
const ACTION_STORAGE_NETWORK: u32 = 20_102;
const ACTION_ADMISSION_DAO: u32 = 21_001;
const ACTION_ADMISSION_DEVICE: u32 = 21_002;
const ACTION_ADMISSION_OWNED: u32 = 21_003;
const ACTION_TPM_DEVICE_KEY: u32 = 21_101;
const ACTION_YUBIKEY_UNLOCK: u32 = 21_102;
const ACTION_FINGERPRINT_UNLOCK: u32 = 21_103;
const ACTION_SEAL_MASTER_KEY: u32 = 21_201;
const ACTION_TEST_UNSEAL: u32 = 21_202;
const ACTION_SYSTEMD_INVENTORY: u32 = 22_001;
const ACTION_STATIC_HTTP: u32 = 22_101;
const ACTION_ACME_CERTS: u32 = 22_102;
const ACTION_DNS_ZONE: u32 = 22_103;
const ACTION_IPXE_BOOT: u32 = 22_201;
const ACTION_TFTP_BOOT: u32 = 22_202;
const ACTION_IMAP_SERVICE: u32 = 22_301;
const ACTION_SMTP_SERVICE: u32 = 22_302;
const ACTION_PROXY_SERVICE: u32 = 22_401;
const ACTION_PASSWORD_VAULT: u32 = 22_501;
const TRUST_SETUP_TOTAL: usize = 9;
const SERVICE_SETUP_TOTAL: usize = 11;
const NODE_KEY_ENVELOPE_FILE: &str = "node-signing-key.envelope";
const NODE_ID_FILE: &str = "node-id.bin";
const TPM_STATUS_FILE: &str = "tpm-device-key.status";
const YUBIKEY_STATUS_FILE: &str = "yubikey-unlock.status";
const FINGERPRINT_STATUS_FILE: &str = "fingerprint-unlock.status";
const PASSWORD_SALT_FILE: &str = "password-root.salt";
const PASSWORD_VERIFIER_FILE: &str = "password-root.verifier";
const PASSWORD_SECRET_FILE: &str = "password-root.envelope";
const PASSWORD_KDF_ROUNDS_FILE: &str = "password-root.kdf-rounds";
const PASSWORD_KDF_ROUNDS: u32 = 120_000;
const PASSWORD_MIN_LEN: usize = 8;
const PASSWORD_SALT_LEN: usize = 32;
const LOCAL_SECRET_LEN: usize = 32;
const PASSWORD_KDF_DOMAIN: &[u8] = b"edgerun:v1:node-ui:password-root:kdf";
const PASSWORD_VERIFIER_DOMAIN: &[u8] = b"edgerun:v1:node-ui:password-root:verifier";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActivePanel {
    MachineReport,
    NodeInstances,
    Storage,
    Trust,
    Services,
}

impl ActivePanel {
    const fn title(self) -> &'static str {
        match self {
            Self::MachineReport => "Machine Report",
            Self::NodeInstances => "Node Instances",
            Self::Storage => "Storage",
            Self::Trust => "Trust",
            Self::Services => "Services",
        }
    }

    const fn subtitle(self) -> &'static str {
        match self {
            Self::MachineReport => "local hardware, capacity, and providers",
            Self::NodeInstances => "role instances, runtimes, and local authority",
            Self::Storage => "content, cache, and object materialization",
            Self::Trust => "identity, policy, and proof state",
            Self::Services => "systemd, web, mail, boot, proxy, DNS, and certs",
        }
    }
}

#[derive(Clone, Debug)]
struct ReportSection {
    title: &'static str,
    detail: String,
    accent: Color4,
    rows: Vec<String>,
}

#[derive(Clone, Debug)]
struct MachineReport {
    refreshed_at: String,
    platform: &'static str,
    cores: String,
    memory: String,
    capability_count: usize,
    device_count: usize,
    sections: Vec<ReportSection>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum AdmissionMode {
    #[default]
    None,
    EdgeRunDao,
    Device,
    Owned,
}

#[derive(Clone, Debug)]
struct NodeControlState {
    admission_mode: AdmissionMode,
    tpm_device_key: bool,
    yubikey_unlock: bool,
    fingerprint_unlock: bool,
    password_root: bool,
    master_key_sealed: bool,
    unseal_tested: bool,
    tpm_status: String,
    yubikey_status: String,
    fingerprint_status: String,
    password_status: String,
    sealed_key_status: String,
    sealed_node_id: Option<[u8; 64]>,
    sealed_node_key: Option<Vec<u8>>,
    seal_key: Option<SealKey>,
    local_secret: Option<[u8; LOCAL_SECRET_LEN]>,
    local_cache: bool,
    network_storage: bool,
    systemd_inventory: bool,
    static_http: bool,
    acme_certs: bool,
    dns_zone: bool,
    ipxe_boot: bool,
    tftp_boot: bool,
    imap_service: bool,
    smtp_service: bool,
    proxy_service: bool,
    password_vault: bool,
    last_event: String,
    events: Vec<String>,
}

impl Default for NodeControlState {
    fn default() -> Self {
        Self {
            admission_mode: AdmissionMode::None,
            tpm_device_key: false,
            yubikey_unlock: false,
            fingerprint_unlock: false,
            password_root: false,
            master_key_sealed: false,
            unseal_tested: false,
            tpm_status: "not provisioned".to_string(),
            yubikey_status: "not enrolled".to_string(),
            fingerprint_status: "not enrolled".to_string(),
            password_status: "not configured".to_string(),
            sealed_key_status: "not sealed".to_string(),
            sealed_node_id: None,
            sealed_node_key: None,
            seal_key: None,
            local_secret: None,
            local_cache: false,
            network_storage: false,
            systemd_inventory: false,
            static_http: false,
            acme_certs: false,
            dns_zone: false,
            ipxe_boot: false,
            tftp_boot: false,
            imap_service: false,
            smtp_service: false,
            proxy_service: false,
            password_vault: false,
            last_event: "No setup action taken".to_string(),
            events: vec!["Node UI opened; no signed setup actions yet".to_string()],
        }
    }
}

#[derive(Clone, Debug)]
struct SealedNodeKey {
    node_id: [u8; 64],
    envelope: Vec<u8>,
    seal_key: SealKey,
}

impl SealedNodeKey {
    fn summary(&self) -> String {
        format!(
            "{} sealed envelope, node {}",
            format_bytes(self.envelope.len() as u64),
            short_hex(&self.node_id, 8)
        )
    }
}

#[derive(Clone, Debug)]
struct PasswordRoot {
    salt: [u8; PASSWORD_SALT_LEN],
    verifier: [u8; LOCAL_SECRET_LEN],
    secret: [u8; LOCAL_SECRET_LEN],
    envelope: Vec<u8>,
    rounds: u32,
}

#[derive(Clone, Debug)]
struct PasswordRootRecord {
    salt: [u8; PASSWORD_SALT_LEN],
    verifier: [u8; LOCAL_SECRET_LEN],
    envelope: Vec<u8>,
    rounds: u32,
}

struct PendingAction {
    label: &'static str,
    receiver: Receiver<ActionResult>,
}

enum ActionResult {
    Tpm(Result<String, String>),
    YubiKey(Result<String, String>),
    Fingerprint(Result<String, String>),
}

fn background_action_label(id: u32) -> Option<&'static str> {
    match id {
        ACTION_TPM_DEVICE_KEY => Some("Creating TPM device key"),
        ACTION_YUBIKEY_UNLOCK => Some("Probing YubiKey unlock"),
        ACTION_FINGERPRINT_UNLOCK => Some("Enrolling fingerprint unlock"),
        _ => None,
    }
}

fn spawn_background_action(id: u32) -> Option<PendingAction> {
    let label = background_action_label(id)?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = match id {
            ACTION_TPM_DEVICE_KEY => ActionResult::Tpm(provision_tpm_device_key()),
            ACTION_YUBIKEY_UNLOCK => ActionResult::YubiKey(probe_yubikey_unlock()),
            ACTION_FINGERPRINT_UNLOCK => ActionResult::Fingerprint(
                crate::hardware::enroll_first_fingerprint_template("EdgeRun device unlock"),
            ),
            _ => return,
        };
        let _ = sender.send(result);
    });
    Some(PendingAction { label, receiver })
}

#[derive(Clone, Debug)]
struct LocalAuthorityStore {
    root: PathBuf,
}

impl LocalAuthorityStore {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn default_root() -> PathBuf {
        std::env::var_os("EDGERUN_NODE_UI_ROOT")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".edgerun").join("node-ui"))
            })
            .unwrap_or_else(|| PathBuf::from("node-ui-data"))
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn load(&self) -> NodeControlState {
        let mut control = NodeControlState::default();
        if let Ok(record) = self.load_password_root_record() {
            control.password_root = true;
            control.password_status = format!(
                "configured, {} envelope, {} KDF rounds",
                format_bytes(record.envelope.len() as u64),
                record.rounds
            );
        }
        if let Ok(summary) = self.read_status(TPM_STATUS_FILE) {
            control.tpm_device_key = true;
            control.tpm_status = summary;
        }
        if let Ok(summary) = self.read_status(YUBIKEY_STATUS_FILE) {
            control.yubikey_unlock = true;
            control.yubikey_status = summary;
        }
        if let Ok(summary) = self.read_status(FINGERPRINT_STATUS_FILE) {
            control.fingerprint_unlock = true;
            control.fingerprint_status = summary;
        }
        if let (Ok(node_id_bytes), Ok(envelope)) = (
            fs::read(self.root.join(NODE_ID_FILE)),
            fs::read(self.root.join(NODE_KEY_ENVELOPE_FILE)),
        ) {
            if node_id_bytes.len() == 64 && !envelope.is_empty() {
                let mut node_id = [0u8; 64];
                node_id.copy_from_slice(&node_id_bytes);
                control.master_key_sealed = true;
                control.sealed_node_id = Some(node_id);
                control.sealed_node_key = Some(envelope);
                control.sealed_key_status = format!(
                    "{} sealed envelope, node {}",
                    format_bytes(control.sealed_node_key.as_ref().map_or(0, Vec::len) as u64),
                    short_hex(&node_id, 8)
                );
            }
        }
        if control.setup_count() > 0 {
            control.record_event_owned(format!(
                "Loaded local authority state from {}",
                self.root.display()
            ));
        }
        control
    }

    fn persist_node_key(&self, sealed: &SealedNodeKey) -> Result<(), String> {
        fs::create_dir_all(&self.root).map_err(|err| {
            format!(
                "create authority root {} failed: {err}",
                self.root.display()
            )
        })?;
        fs::write(self.root.join(NODE_ID_FILE), sealed.node_id)
            .map_err(|err| format!("write node id failed: {err}"))?;
        fs::write(self.root.join(NODE_KEY_ENVELOPE_FILE), &sealed.envelope)
            .map_err(|err| format!("write sealed node key envelope failed: {err}"))?;
        Ok(())
    }

    fn persist_password_root(&self, root: &PasswordRoot) -> Result<(), String> {
        fs::create_dir_all(&self.root).map_err(|err| {
            format!(
                "create authority root {} failed: {err}",
                self.root.display()
            )
        })?;
        fs::write(self.root.join(PASSWORD_SALT_FILE), root.salt)
            .map_err(|err| format!("write password salt failed: {err}"))?;
        fs::write(self.root.join(PASSWORD_VERIFIER_FILE), root.verifier)
            .map_err(|err| format!("write password verifier failed: {err}"))?;
        fs::write(self.root.join(PASSWORD_SECRET_FILE), &root.envelope)
            .map_err(|err| format!("write password secret envelope failed: {err}"))?;
        fs::write(
            self.root.join(PASSWORD_KDF_ROUNDS_FILE),
            root.rounds.to_string().as_bytes(),
        )
        .map_err(|err| format!("write password KDF rounds failed: {err}"))?;
        Ok(())
    }

    fn load_password_root_record(&self) -> Result<PasswordRootRecord, String> {
        let salt_bytes = fs::read(self.root.join(PASSWORD_SALT_FILE))
            .map_err(|err| format!("read password salt failed: {err}"))?;
        let verifier_bytes = fs::read(self.root.join(PASSWORD_VERIFIER_FILE))
            .map_err(|err| format!("read password verifier failed: {err}"))?;
        let envelope = fs::read(self.root.join(PASSWORD_SECRET_FILE))
            .map_err(|err| format!("read password secret envelope failed: {err}"))?;
        let rounds = fs::read_to_string(self.root.join(PASSWORD_KDF_ROUNDS_FILE))
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok())
            .unwrap_or(PASSWORD_KDF_ROUNDS);
        if salt_bytes.len() != PASSWORD_SALT_LEN {
            return Err("password salt has invalid length".to_string());
        }
        if verifier_bytes.len() != LOCAL_SECRET_LEN {
            return Err("password verifier has invalid length".to_string());
        }
        if envelope.is_empty() {
            return Err("password secret envelope is empty".to_string());
        }
        let mut salt = [0u8; PASSWORD_SALT_LEN];
        salt.copy_from_slice(&salt_bytes);
        let mut verifier = [0u8; LOCAL_SECRET_LEN];
        verifier.copy_from_slice(&verifier_bytes);
        Ok(PasswordRootRecord {
            salt,
            verifier,
            envelope,
            rounds,
        })
    }

    fn unlock_password_root(&self, password: &str) -> Result<[u8; LOCAL_SECRET_LEN], String> {
        let record = self.load_password_root_record()?;
        unlock_password_root_record(password, &record)
    }

    fn persist_status(&self, file: &str, status: &str) -> Result<(), String> {
        fs::create_dir_all(&self.root).map_err(|err| {
            format!(
                "create authority root {} failed: {err}",
                self.root.display()
            )
        })?;
        fs::write(self.root.join(file), status.as_bytes())
            .map_err(|err| format!("write status {file} failed: {err}"))
    }

    fn read_status(&self, file: &str) -> Result<String, String> {
        fs::read_to_string(self.root.join(file))
            .map(|value| value.trim().to_string())
            .map_err(|err| format!("read status {file} failed: {err}"))
    }
}

fn generate_and_seal_node_key() -> Result<SealedNodeKey, String> {
    let (signing_key, identity) = generate_node_signing_key();
    let seal_key =
        generate_seal_key().map_err(|err| format!("seal key generation failed: {err:?}"))?;
    let envelope = seal_node_signing_key(&signing_key, &seal_key)
        .map_err(|err| format!("node key sealing failed: {err:?}"))?;
    Ok(SealedNodeKey {
        node_id: identity.node_id,
        envelope,
        seal_key,
    })
}

#[cfg(all(feature = "all-hardware", target_os = "linux"))]
fn provision_tpm_device_key() -> Result<String, String> {
    use edgerun_tpm::{LinuxTpmDevice, LinuxTpmSigningKey, TpmDevice, TpmHandle, TpmSigningKey};

    const HANDLE: u32 = 0x8100_0001;
    let mut last_error = String::new();
    for path in ["/dev/tpmrm0", "/dev/tpm0"] {
        let mut tpm = TpmDevice::new(LinuxTpmDevice::new(path));
        match tpm.create_ecdsa_p256_signing_key(HANDLE) {
            Ok(key) => {
                return Ok(format!(
                    "{path} handle 0x{:08x}, pub {}, name {}",
                    key.persistent_handle,
                    format_bytes(key.public_key_bytes.len() as u64),
                    short_hex(&key.name, 6)
                ));
            }
            Err(create_err) => {
                let signing_key = LinuxTpmSigningKey::new(path, TpmHandle(HANDLE));
                match signing_key.key_info() {
                    Ok(info) => {
                        return Ok(format!(
                            "{path} existing handle 0x{HANDLE:08x}, {:?}, pub {}",
                            info.algorithm,
                            format_bytes(info.public_key.len() as u64)
                        ));
                    }
                    Err(read_err) => {
                        last_error = format!(
                            "{path}: create failed ({create_err}); read failed ({read_err})"
                        );
                    }
                }
            }
        }
    }
    Err(if last_error.is_empty() {
        "no TPM device path responded".to_string()
    } else {
        last_error
    })
}

#[cfg(not(all(feature = "all-hardware", target_os = "linux")))]
fn provision_tpm_device_key() -> Result<String, String> {
    Err("TPM device key provisioning requires Linux all-hardware support".to_string())
}

#[cfg(all(feature = "all-hardware", target_os = "linux"))]
fn probe_yubikey_unlock() -> Result<String, String> {
    use edgerun_yubikey::{LinuxPcscYubiKey, LinuxUsbYubiKey, YubiKeyPivSlot};

    let devices =
        LinuxUsbYubiKey::discover().map_err(|err| format!("YubiKey discovery failed: {err}"))?;
    let device = devices
        .into_iter()
        .next()
        .ok_or_else(|| "no YubiKey USB device found".to_string())?;
    let key = LinuxPcscYubiKey::new(device.clone(), YubiKeyPivSlot::Authentication);
    let capabilities = key
        .probe_capabilities()
        .map_err(|err| format!("YubiKey PIV probe failed: {err}"))?;
    Ok(format!(
        "bus {} dev {}, PIV={}, attestation={}, metadata={}",
        device.bus,
        device.device,
        capabilities.piv_applet,
        capabilities.slot_attestation || capabilities.attestation_certificate,
        capabilities.slot_metadata
    ))
}

#[cfg(not(all(feature = "all-hardware", target_os = "linux")))]
fn probe_yubikey_unlock() -> Result<String, String> {
    Err("YubiKey unlock probing requires Linux all-hardware support".to_string())
}

fn short_hex(bytes: &[u8], count: usize) -> String {
    let mut out = String::from("0x");
    for byte in bytes.iter().take(count) {
        out.push(hex_digit(byte >> 4));
        out.push(hex_digit(byte & 0x0f));
    }
    out
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        _ => (b'a' + (value - 10)) as char,
    }
}

impl NodeControlState {
    fn apply_action(&mut self, id: u32) -> bool {
        match id {
            ACTION_STORAGE_LOCAL_CACHE => {
                self.local_cache = true;
                self.record_event("Local cache path marked for setup");
                true
            }
            ACTION_STORAGE_NETWORK => {
                self.network_storage = true;
                self.record_event("Network storage admission request staged");
                true
            }
            ACTION_ADMISSION_DAO => {
                self.admission_mode = AdmissionMode::EdgeRunDao;
                self.record_event("EdgeRun DAO admission selected");
                true
            }
            ACTION_ADMISSION_DEVICE => {
                self.admission_mode = AdmissionMode::Device;
                self.record_event("Device admission authority staged");
                true
            }
            ACTION_ADMISSION_OWNED => {
                self.admission_mode = AdmissionMode::Owned;
                self.record_event("Owned admission authority staged");
                true
            }
            ACTION_TPM_DEVICE_KEY => match provision_tpm_device_key() {
                Ok(summary) => {
                    self.tpm_device_key = true;
                    self.tpm_status = summary.clone();
                    self.record_event_owned(format!("TPM device key ready: {summary}"));
                    true
                }
                Err(err) => {
                    self.tpm_device_key = false;
                    self.tpm_status = err.clone();
                    self.record_event_owned(format!("TPM device key failed: {err}"));
                    false
                }
            },
            ACTION_YUBIKEY_UNLOCK => match probe_yubikey_unlock() {
                Ok(summary) => {
                    self.yubikey_unlock = true;
                    self.yubikey_status = summary.clone();
                    self.record_event_owned(format!("YubiKey unlock ready: {summary}"));
                    true
                }
                Err(err) => {
                    self.yubikey_unlock = false;
                    self.yubikey_status = err.clone();
                    self.record_event_owned(format!("YubiKey unlock failed: {err}"));
                    false
                }
            },
            ACTION_FINGERPRINT_UNLOCK => {
                match crate::hardware::enroll_first_fingerprint_template("EdgeRun device unlock") {
                    Ok(summary) => {
                        self.fingerprint_unlock = true;
                        self.fingerprint_status = summary.clone();
                        self.record_event_owned(format!("Fingerprint unlock enrolled: {summary}"));
                        true
                    }
                    Err(err) => {
                        self.fingerprint_unlock = false;
                        self.fingerprint_status = err.clone();
                        self.record_event_owned(format!("Fingerprint enrollment failed: {err}"));
                        false
                    }
                }
            }
            ACTION_SEAL_MASTER_KEY => match generate_and_seal_node_key() {
                Ok(sealed) => {
                    self.master_key_sealed = true;
                    self.unseal_tested = false;
                    let node_id = sealed.node_id;
                    let envelope_len = sealed.envelope.len();
                    self.sealed_key_status = sealed.summary();
                    self.sealed_node_id = Some(node_id);
                    self.sealed_node_key = Some(sealed.envelope);
                    self.seal_key = Some(sealed.seal_key);
                    self.record_event_owned(format!(
                        "Generated node key {} and sealed {} bytes",
                        short_hex(&node_id, 8),
                        envelope_len
                    ));
                    true
                }
                Err(err) => {
                    self.master_key_sealed = false;
                    self.sealed_key_status = err.clone();
                    self.record_event_owned(format!("Key generation/seal failed: {err}"));
                    false
                }
            },
            ACTION_TEST_UNSEAL => match self.test_unseal() {
                Ok(summary) => {
                    self.unseal_tested = true;
                    self.record_event_owned(format!("Sealed node key unseal passed: {summary}"));
                    true
                }
                Err(err) => {
                    self.unseal_tested = false;
                    self.record_event_owned(format!("Sealed node key unseal failed: {err}"));
                    false
                }
            },
            ACTION_SYSTEMD_INVENTORY => {
                self.systemd_inventory = true;
                self.record_event("Systemd service inventory staged");
                true
            }
            ACTION_STATIC_HTTP => {
                self.static_http = true;
                self.record_event("Static HTTP publishing service staged");
                true
            }
            ACTION_ACME_CERTS => {
                self.acme_certs = true;
                self.record_event("ACME certificate automation staged");
                true
            }
            ACTION_DNS_ZONE => {
                self.dns_zone = true;
                self.record_event("DNS zone authority staged");
                true
            }
            ACTION_IPXE_BOOT => {
                self.ipxe_boot = true;
                self.record_event("iPXE boot service staged");
                true
            }
            ACTION_TFTP_BOOT => {
                self.tftp_boot = true;
                self.record_event("TFTP boot service staged");
                true
            }
            ACTION_IMAP_SERVICE => {
                self.imap_service = true;
                self.record_event("IMAP mailbox service staged");
                true
            }
            ACTION_SMTP_SERVICE => {
                self.smtp_service = true;
                self.record_event("SMTP submission service staged");
                true
            }
            ACTION_PROXY_SERVICE => {
                self.proxy_service = true;
                self.record_event("Proxy route service staged");
                true
            }
            ACTION_PASSWORD_VAULT => {
                self.password_vault = true;
                self.record_event("Password vault management staged");
                true
            }
            _ => false,
        }
    }

    fn record_event(&mut self, event: &str) {
        self.last_event = event.to_string();
        self.events.insert(0, event.to_string());
        self.events.truncate(6);
    }

    fn record_event_owned(&mut self, event: String) {
        self.last_event = event.clone();
        self.events.insert(0, event);
        self.events.truncate(6);
    }

    fn apply_background_result(&mut self, result: ActionResult) {
        match result {
            ActionResult::Tpm(result) => match result {
                Ok(summary) => {
                    self.tpm_device_key = true;
                    self.tpm_status = summary.clone();
                    self.record_event_owned(format!("TPM device key ready: {summary}"));
                }
                Err(err) => {
                    self.tpm_device_key = false;
                    self.tpm_status = err.clone();
                    self.record_event_owned(format!("TPM device key failed: {err}"));
                }
            },
            ActionResult::YubiKey(result) => match result {
                Ok(summary) => {
                    self.yubikey_unlock = true;
                    self.yubikey_status = summary.clone();
                    self.record_event_owned(format!("YubiKey unlock ready: {summary}"));
                }
                Err(err) => {
                    self.yubikey_unlock = false;
                    self.yubikey_status = err.clone();
                    self.record_event_owned(format!("YubiKey unlock failed: {err}"));
                }
            },
            ActionResult::Fingerprint(result) => match result {
                Ok(summary) => {
                    self.fingerprint_unlock = true;
                    self.fingerprint_status = summary.clone();
                    self.record_event_owned(format!("Fingerprint unlock enrolled: {summary}"));
                }
                Err(err) => {
                    self.fingerprint_unlock = false;
                    self.fingerprint_status = err.clone();
                    self.record_event_owned(format!("Fingerprint enrollment failed: {err}"));
                }
            },
        }
    }

    fn test_unseal(&self) -> Result<String, String> {
        let envelope = self
            .sealed_node_key
            .as_ref()
            .ok_or_else(|| "no sealed node key envelope exists".to_string())?;
        let seal_key = self
            .seal_key
            .as_ref()
            .ok_or_else(|| "no in-memory seal key available for this session".to_string())?;
        let expected = self
            .sealed_node_id
            .as_ref()
            .ok_or_else(|| "sealed node id missing".to_string())?;
        let signing_key = unseal_node_signing_key(envelope, seal_key)
            .map_err(|err| format!("unseal failed: {err:?}"))?;
        let node_id = node_id_from_signing_key(&signing_key);
        if &node_id != expected {
            return Err(format!(
                "unsealed key id {} did not match sealed id {}",
                short_hex(&node_id, 8),
                short_hex(expected, 8)
            ));
        }
        Ok(short_hex(&node_id, 8))
    }

    const fn admission_label(&self) -> &'static str {
        match self.admission_mode {
            AdmissionMode::None => "not configured",
            AdmissionMode::EdgeRunDao => "EdgeRun DAO",
            AdmissionMode::Device => "device",
            AdmissionMode::Owned => "owned",
        }
    }

    const fn admission_detail(&self) -> &'static str {
        match self.admission_mode {
            AdmissionMode::None => "device policy authority",
            AdmissionMode::EdgeRunDao => "default admission selected",
            AdmissionMode::Device => "local admission staged",
            AdmissionMode::Owned => "external authority staged",
        }
    }

    fn setup_count(&self) -> usize {
        usize::from(self.admission_mode != AdmissionMode::None)
            + usize::from(self.tpm_device_key)
            + usize::from(self.yubikey_unlock)
            + usize::from(self.fingerprint_unlock)
            + usize::from(self.master_key_sealed)
            + usize::from(self.unseal_tested)
            + usize::from(self.local_cache)
            + usize::from(self.network_storage)
    }

    fn service_count(&self) -> usize {
        usize::from(self.systemd_inventory)
            + usize::from(self.static_http)
            + usize::from(self.acme_certs)
            + usize::from(self.dns_zone)
            + usize::from(self.ipxe_boot)
            + usize::from(self.tftp_boot)
            + usize::from(self.imap_service)
            + usize::from(self.smtp_service)
            + usize::from(self.proxy_service)
            + usize::from(self.password_vault)
            + usize::from(self.network_storage)
    }

    fn next_step(&self) -> &'static str {
        if self.admission_mode == AdmissionMode::None {
            "choose admission authority"
        } else if !self.tpm_device_key {
            "create TPM device key"
        } else if !self.yubikey_unlock && !self.fingerprint_unlock {
            "enroll unlock factor"
        } else if !self.master_key_sealed {
            "seal node signing key"
        } else if !self.unseal_tested {
            "test unseal"
        } else if !self.local_cache {
            "add local cache"
        } else if !self.network_storage {
            "attach storage route"
        } else {
            "ready for signed setup"
        }
    }

    fn readiness_label(&self) -> &'static str {
        if self.setup_count() == 0 {
            "not started"
        } else if self.setup_count() < TRUST_SETUP_TOTAL / 2 {
            "in progress"
        } else if self.setup_count() < TRUST_SETUP_TOTAL {
            "needs review"
        } else {
            "ready"
        }
    }

    fn service_next_step(&self) -> &'static str {
        if !self.systemd_inventory {
            "inspect systemd units"
        } else if !self.dns_zone {
            "stage DNS authority"
        } else if !self.acme_certs {
            "create ACME certificates"
        } else if !self.static_http {
            "set up static HTTP"
        } else if !self.proxy_service {
            "stage proxy routes"
        } else if !self.ipxe_boot {
            "stage iPXE boot"
        } else if !self.tftp_boot {
            "stage TFTP service"
        } else if !self.smtp_service {
            "stage SMTP submission"
        } else if !self.imap_service {
            "stage IMAP mailbox"
        } else if !self.password_vault {
            "stage password vault"
        } else {
            "ready for signed service apply"
        }
    }

    fn service_apply_status(&self) -> &'static str {
        if self.service_blockers().is_empty() {
            "ready"
        } else {
            "blocked"
        }
    }

    fn service_blockers(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.admission_mode == AdmissionMode::None {
            blockers.push("choose admission before exposing services".to_string());
        }
        if !self.tpm_device_key {
            blockers.push("create device key before writing certs/secrets".to_string());
        }
        if self.acme_certs && !self.dns_zone {
            blockers.push("DNS zone required before ACME automation".to_string());
        }
        if (self.static_http || self.proxy_service) && !self.acme_certs {
            blockers.push("TLS certificate policy required for public routes".to_string());
        }
        if (self.imap_service || self.smtp_service || self.password_vault)
            && !self.master_key_sealed
        {
            blockers.push("seal node signing key before storing service secrets".to_string());
        }
        blockers
    }

    fn service_apply_queue(&self) -> Vec<String> {
        let mut queue = Vec::new();
        if self.systemd_inventory {
            queue.push("systemd: inspect and stage EdgeRun units".to_string());
        }
        if self.dns_zone {
            queue.push("dns: publish identity-routed zone records".to_string());
        }
        if self.acme_certs {
            queue.push("acme: create or renew TLS certificates".to_string());
        }
        if self.static_http {
            queue.push("http: serve static verified package/site bytes".to_string());
        }
        if self.proxy_service {
            queue.push("proxy: bind route policy to local service".to_string());
        }
        if self.ipxe_boot {
            queue.push("ipxe: serve boot script and kernel args".to_string());
        }
        if self.tftp_boot {
            queue.push("tftp: serve provisioning artifacts".to_string());
        }
        if self.smtp_service {
            queue.push("smtp: stage submission and delivery policy".to_string());
        }
        if self.imap_service {
            queue.push("imap: stage mailbox access policy".to_string());
        }
        if self.password_vault {
            queue.push("secrets: stage password vault records".to_string());
        }
        if queue.is_empty() {
            queue.push("no host service changes staged".to_string());
        }
        queue
    }
}

pub fn run_window() -> Result<(), String> {
    run_window_for_frames(None)
}

pub fn run_window_for_frames(frames: Option<u32>) -> Result<(), String> {
    run_window_for_frames_with_root(frames, LocalAuthorityStore::default_root())
}

pub fn run_window_for_frames_with_root(
    frames: Option<u32>,
    authority_root: PathBuf,
) -> Result<(), String> {
    let store = LocalAuthorityStore::new(authority_root);
    let state = RefCell::new(NodeUiState {
        report: MachineReport::collect(),
        active_panel: ActivePanel::MachineReport,
        control: store.load(),
        store,
        pending_action: None,
        scroll_y: 0.0,
    });
    run_sdl_gl_window(
        SdlGlWindowOptions::new("EdgeRun Node", 1180, 760, BG)
            .min_size(760, 520)
            .frames(frames),
        |event| match event {
            SdlInputEvent::CloseRequested => SdlEventResult::quit(),
            SdlInputEvent::Tick => {
                if state.borrow_mut().poll_pending_action() {
                    SdlEventResult::dirty()
                } else {
                    SdlEventResult::default()
                }
            }
            SdlInputEvent::KeyDown {
                key: SDL_KEY_ESCAPE,
            } => SdlEventResult::quit(),
            SdlInputEvent::KeyDown { key } if key == SDLK_R || key == SDLK_R - 32 => {
                state.borrow_mut().report = MachineReport::collect();
                SdlEventResult::dirty()
            }
            SdlInputEvent::UiAction {
                action: UiAction::Activated(hit),
            } if hit.kind == HitKind::MenuItem => {
                let mut state = state.borrow_mut();
                let Some(active_panel) = nav_panel_for_id(hit.id) else {
                    return SdlEventResult::default();
                };
                if state.active_panel != active_panel {
                    state.active_panel = active_panel;
                    state.scroll_y = 0.0;
                    SdlEventResult::dirty()
                } else {
                    SdlEventResult::default()
                }
            }
            SdlInputEvent::UiAction {
                action: UiAction::Activated(hit),
            } if hit.kind == HitKind::Button => {
                let mut state = state.borrow_mut();
                if state.pending_action.is_some() {
                    state
                        .control
                        .record_event("Another hardware action is already running");
                    return SdlEventResult::dirty();
                }
                if let Some(pending) = spawn_background_action(hit.id) {
                    state
                        .control
                        .record_event_owned(format!("{} started", pending.label));
                    state.pending_action = Some(pending);
                    return SdlEventResult::dirty();
                }
                if state.control.apply_action(hit.id) {
                    if hit.id == ACTION_SEAL_MASTER_KEY {
                        if let Err(err) = state.persist_current_node_key() {
                            state.control.record_event_owned(format!(
                                "Persist sealed node key failed: {err}"
                            ));
                        }
                    }
                    SdlEventResult::dirty()
                } else {
                    SdlEventResult::default()
                }
            }
            SdlInputEvent::MouseWheel { y } => {
                let mut state = state.borrow_mut();
                state.scroll_y = (state.scroll_y - y * 52.0).max(0.0);
                SdlEventResult::dirty()
            }
            SdlInputEvent::Resized { .. } => SdlEventResult::dirty(),
            _ => SdlEventResult::default(),
        },
        |width, height| {
            let state = state.borrow();
            build_layout(
                &state.report,
                &state.control,
                state.active_panel,
                width as f32,
                height as f32,
                state.scroll_y,
            )
        },
    )
}

struct NodeUiState {
    report: MachineReport,
    active_panel: ActivePanel,
    control: NodeControlState,
    store: LocalAuthorityStore,
    pending_action: Option<PendingAction>,
    scroll_y: f32,
}

impl NodeUiState {
    fn poll_pending_action(&mut self) -> bool {
        let Some(pending) = self.pending_action.as_ref() else {
            return false;
        };
        match pending.receiver.try_recv() {
            Ok(result) => {
                let status_file = match &result {
                    ActionResult::Tpm(_) => TPM_STATUS_FILE,
                    ActionResult::YubiKey(_) => YUBIKEY_STATUS_FILE,
                    ActionResult::Fingerprint(_) => FINGERPRINT_STATUS_FILE,
                };
                self.control.apply_background_result(result);
                let status = match status_file {
                    TPM_STATUS_FILE if self.control.tpm_device_key => {
                        Some(self.control.tpm_status.as_str())
                    }
                    YUBIKEY_STATUS_FILE if self.control.yubikey_unlock => {
                        Some(self.control.yubikey_status.as_str())
                    }
                    FINGERPRINT_STATUS_FILE if self.control.fingerprint_unlock => {
                        Some(self.control.fingerprint_status.as_str())
                    }
                    _ => None,
                };
                if let Some(status) = status {
                    if let Err(err) = self.store.persist_status(status_file, status) {
                        self.control
                            .record_event_owned(format!("Persist hardware status failed: {err}"));
                    }
                }
                self.pending_action = None;
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                let label = pending.label;
                self.control
                    .record_event_owned(format!("{label} failed: worker disconnected"));
                self.pending_action = None;
                true
            }
        }
    }

    fn persist_current_node_key(&mut self) -> Result<(), String> {
        let (Some(node_id), Some(envelope), Some(seal_key)) = (
            self.control.sealed_node_id,
            self.control.sealed_node_key.clone(),
            self.control.seal_key,
        ) else {
            return Err("no sealed node key to persist".to_string());
        };
        let sealed = SealedNodeKey {
            node_id,
            envelope,
            seal_key,
        };
        self.store.persist_node_key(&sealed)?;
        self.control.record_event_owned(format!(
            "Persisted sealed node key envelope to {}",
            self.store.root().display()
        ));
        Ok(())
    }
}

impl MachineReport {
    fn collect() -> Self {
        let inventory = HardwareInventory::discover();
        let capacity = NodeCapacity::discover();
        let sections = section_list(&inventory);
        let device_count = sections.iter().map(|section| section.rows.len()).sum();

        Self {
            refreshed_at: refresh_label(),
            platform: inventory.platform,
            cores: capacity.total_cores.to_string(),
            memory: format_bytes(capacity.total_memory_bytes),
            capability_count: inventory.capability_descriptors.len(),
            device_count,
            sections,
        }
    }
}

fn section_list(inventory: &HardwareInventory) -> Vec<ReportSection> {
    vec![
        section(
            "Compute",
            "GPU and accelerator devices",
            ACCENT,
            merge_rows(&[&inventory.gpus, &inventory.npu_devices]),
        ),
        section(
            "Displays",
            "DRM connectors and output state",
            BLUE,
            inventory.displays.clone(),
        ),
        section(
            "Network",
            "WiFi, Bluetooth, NFC, and CEC",
            ACCENT,
            merge_rows(&[
                &inventory.wifi_interfaces,
                &inventory.bluetooth_controllers,
                &inventory.nfc_adapters,
                &inventory.cec_adapters,
            ]),
        ),
        section(
            "Input",
            "Human input and biometric devices",
            AMBER,
            merge_rows(&[
                &inventory.input_devices,
                &inventory.fingerprint_readers,
                &inventory.biometric,
            ]),
        ),
        section(
            "Media",
            "Audio capture, output, and camera",
            BLUE,
            merge_rows(&[
                &inventory.audio_input,
                &inventory.audio_output,
                &inventory.camera,
                &inventory.sensors,
            ]),
        ),
        section(
            "Buses",
            "USB and PCI inventory",
            ACCENT,
            merge_rows(&[&inventory.usb_devices, &inventory.pci_devices]),
        ),
        section(
            "Power",
            "Battery and power supply state",
            AMBER,
            inventory.power_supplies.clone(),
        ),
        section(
            "Authority",
            "Keystore and capability providers",
            ACCENT,
            merge_rows(&[&inventory.keystore, &inventory.location]),
        ),
    ]
}

fn section(title: &'static str, detail: &str, accent: Color4, rows: Vec<String>) -> ReportSection {
    ReportSection {
        title,
        detail: detail.to_string(),
        accent,
        rows,
    }
}

fn merge_rows(groups: &[&Vec<String>]) -> Vec<String> {
    let mut rows = Vec::new();
    for group in groups {
        rows.extend(group.iter().cloned());
    }
    rows
}

fn build_layout(
    report: &MachineReport,
    control: &NodeControlState,
    active_panel: ActivePanel,
    width: f32,
    _height: f32,
    scroll_y: f32,
) -> UiNode {
    let content_w = (width - 248.0).max(420.0);
    UiNode::row("w-full h-full bg-bg")
        .child(
            sidebar_layout(report, control, active_panel)
                .class("w-62 h-full bg-sidebar border p-6 gap-2"),
        )
        .child(
            UiNode::column("flex-1 h-full bg-bg")
                .child(
                    header_layout(report, active_panel)
                        .class("h-19 w-full bg-topbar border px-7 py-4"),
                )
                .child(
                    UiNode::scroll_area_px("flex-1 w-full overflow-hidden p-7 gap-6", scroll_y)
                        .child(panel_body(report, control, active_panel, content_w))
                        .child(
                            UiNode::text("Press R to refresh. Esc closes the native node UI.")
                                .class("h-6 text-muted"),
                        ),
                ),
        )
}

fn sidebar_layout(
    report: &MachineReport,
    control: &NodeControlState,
    active_panel: ActivePanel,
) -> UiNode {
    UiNode::column("w-full h-full gap-2")
        .child(
            UiNode::row("h-12 w-full items-center gap-3")
                .child(UiNode::icon(UiIcon::Server).class("size-7 text-accent"))
                .child(
                    UiNode::column("flex-1 gap-1")
                        .child(UiNode::text("edgerun node").class("h-5 text-text truncate"))
                        .child(UiNode::text("machine authority").class("h-5 text-muted truncate")),
                ),
        )
        .child(UiNode::divider("w-full"))
        .child(nav_item(
            "Machine report",
            "hardware and providers",
            NAV_MACHINE_REPORT_ID,
            active_panel == ActivePanel::MachineReport,
        ))
        .child(nav_item(
            "Node instances",
            "roles and runtimes",
            NAV_NODE_INSTANCES_ID,
            active_panel == ActivePanel::NodeInstances,
        ))
        .child(nav_item(
            "Storage",
            "objects and cache",
            NAV_STORAGE_ID,
            active_panel == ActivePanel::Storage,
        ))
        .child(nav_item(
            "Trust",
            "policy and proofs",
            NAV_TRUST_ID,
            active_panel == ActivePanel::Trust,
        ))
        .child(nav_item(
            "Services",
            "host control plane",
            NAV_SERVICES_ID,
            active_panel == ActivePanel::Services,
        ))
        .child(UiNode::spacer("flex-1"))
        .child(UiNode::divider("w-full"))
        .child(summary_row("platform", report.platform))
        .child(summary_row("devices", &report.device_count.to_string()))
        .child(summary_row(
            "setup",
            &format!("{}/{}", control.setup_count(), TRUST_SETUP_TOTAL),
        ))
        .child(summary_row(
            "services",
            &format!("{}/{}", control.service_count(), SERVICE_SETUP_TOTAL),
        ))
        .child(summary_row(
            "providers",
            &report.capability_count.to_string(),
        ))
}

fn nav_panel_for_id(id: u32) -> Option<ActivePanel> {
    match id {
        NAV_MACHINE_REPORT_ID => Some(ActivePanel::MachineReport),
        NAV_NODE_INSTANCES_ID => Some(ActivePanel::NodeInstances),
        NAV_STORAGE_ID => Some(ActivePanel::Storage),
        NAV_TRUST_ID => Some(ActivePanel::Trust),
        NAV_SERVICES_ID => Some(ActivePanel::Services),
        _ => None,
    }
}

fn nav_item(label: &str, detail: &str, id: u32, active: bool) -> UiNode {
    UiNode::menu_item(label, id)
        .detail(detail)
        .accent(if active { ACCENT } else { BLUE })
        .selected(active)
        .class("h-10 w-full")
}

fn summary_row(label: &str, value: &str) -> UiNode {
    UiNode::row("h-7 w-full items-center justify-between gap-2")
        .child(UiNode::text(label).class("h-5 text-muted truncate"))
        .child(UiNode::text(value).class("h-5 text-text truncate"))
}

fn header_layout(report: &MachineReport, active_panel: ActivePanel) -> UiNode {
    UiNode::row("w-full h-full items-center justify-between gap-4")
        .child(
            UiNode::column("flex-1 gap-1")
                .child(UiNode::text(active_panel.title()).class("h-5 text-text truncate"))
                .child(UiNode::text(active_panel.subtitle()).class("h-5 text-muted truncate")),
        )
        .child(UiNode::badge("SDL native", BLUE).class("h-6"))
        .child(UiNode::badge(&report.refreshed_at, ACCENT).class("h-6"))
}

fn panel_body(
    report: &MachineReport,
    control: &NodeControlState,
    active_panel: ActivePanel,
    width: f32,
) -> UiNode {
    match active_panel {
        ActivePanel::MachineReport => machine_report_panel(report, width),
        ActivePanel::NodeInstances => node_instances_panel(report, control, width),
        ActivePanel::Storage => storage_panel(report, control, width),
        ActivePanel::Trust => trust_panel(report, control, width),
        ActivePanel::Services => services_panel(report, control, width),
    }
}

fn machine_report_panel(report: &MachineReport, width: f32) -> UiNode {
    UiNode::column("w-full gap-6")
        .child(metric_grid_layout(report, width))
        .child(section_grid_layout(report, width))
}

fn node_instances_panel(report: &MachineReport, control: &NodeControlState, width: f32) -> UiNode {
    UiNode::column("w-full gap-6")
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-3 gap-4", width)
                .class("w-full")
                .child(
                    metric_node(
                        "Admission",
                        control.admission_label(),
                        control.admission_detail(),
                        ACCENT,
                    )
                    .progress(0.25),
                )
                .child(metric_node(
                    "Relay",
                    "not set",
                    "assigned packet path",
                    AMBER,
                ))
                .child(metric_node(
                    "Storage",
                    if control.local_cache {
                        "local cache"
                    } else {
                        "not set"
                    },
                    "content-addressed cache",
                    BLUE,
                )),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Role Instances",
                    "Native and browser-capable roles that this device can run.",
                    UiIcon::Server,
                    ACCENT,
                    &[
                        control_value("admission:device", "local policy authority", "planned"),
                        control_value("relay:private-devices", "device mesh ingress", "not set"),
                        control_value(
                            "storage:local-cache",
                            "verified package/object cache",
                            if control.local_cache {
                                "staged"
                            } else {
                                "not set"
                            },
                        ),
                        control_value(
                            "notary:device",
                            "TPM-backed sealing boundary",
                            if control.tpm_device_key {
                                "staged"
                            } else {
                                "planned"
                            },
                        ),
                    ],
                ))
                .child(control_card(
                    "Hardware Providers",
                    "Discovered local capabilities that can back node roles.",
                    UiIcon::Cpu,
                    BLUE,
                    &[
                        control_value(
                            "hardware devices",
                            "from machine report",
                            &report.device_count.to_string(),
                        ),
                        control_value(
                            "capability providers",
                            "runtime descriptors",
                            &report.capability_count.to_string(),
                        ),
                        control_value("platform", "native target", report.platform),
                    ],
                )),
        )
}

fn storage_panel(report: &MachineReport, control: &NodeControlState, width: f32) -> UiNode {
    UiNode::column("w-full gap-6")
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-4 gap-4", width)
                .class("w-full")
                .child(metric_node("CAS", "not set", "object root", AMBER))
                .child(metric_node(
                    "Cache",
                    if control.local_cache {
                        "staged"
                    } else {
                        "unknown"
                    },
                    "verified bytes",
                    BLUE,
                ))
                .child(metric_node(
                    "Relay Route",
                    if control.network_storage {
                        "staged"
                    } else {
                        "required"
                    },
                    "all movement goes through relay",
                    ACCENT,
                ))
                .child(metric_node(
                    "Materialize",
                    "manual",
                    "object to filesystem",
                    BLUE,
                )),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Signed Apply Queue",
                    "Host mutations staged by this UI before policy approval.",
                    UiIcon::Check,
                    ACCENT,
                    &[
                        control_value(
                            "apply status",
                            "requires admission, device key, and service policy",
                            control.service_apply_status(),
                        ),
                        control_value(
                            "queued changes",
                            "systemd/service operations waiting for approval",
                            &control.service_apply_queue().len().to_string(),
                        ),
                        control_value(
                            "execution mode",
                            "dry-run first; signed apply later",
                            "guarded",
                        ),
                    ],
                ))
                .child(list_card(
                    "Apply Blockers",
                    "Requirements that must be solved before host changes run.",
                    UiIcon::Warning,
                    AMBER,
                    control.service_blockers(),
                    "no blockers",
                )),
        )
        .child(list_card(
            "Queued Host Changes",
            "Concrete service operations this device would apply after approval.",
            UiIcon::File,
            BLUE,
            control.service_apply_queue(),
            "no host service changes staged",
        ))
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Storage Nodes",
                    "Configure where packets, sealed objects, and verified caches live.",
                    UiIcon::Storage,
                    BLUE,
                    &[
                        control_button(
                            "Add local cache path",
                            "choose folder for verified objects",
                            ACTION_STORAGE_LOCAL_CACHE,
                            if control.local_cache {
                                "Staged"
                            } else {
                                "Set up"
                            },
                        ),
                        control_button(
                            "Attach network storage",
                            "request admission before sending packets",
                            ACTION_STORAGE_NETWORK,
                            if control.network_storage {
                                "Staged"
                            } else {
                                "Set up"
                            },
                        ),
                        control_value(
                            "sealing boundary",
                            "VFS must not hold sealing keys",
                            "notary",
                        ),
                    ],
                ))
                .child(control_card(
                    "Object Flow",
                    "Content should stay addressable without forcing plaintext assembly.",
                    UiIcon::Route,
                    ACCENT,
                    &[
                        control_value(
                            "object to packets",
                            "BLAKE3-addressed packet stream",
                            "planned",
                        ),
                        control_value(
                            "packets to object",
                            "reassemble after relay delivery",
                            "planned",
                        ),
                        control_value(
                            "object to file",
                            "materialize only on explicit request",
                            "planned",
                        ),
                    ],
                )),
        )
        .child(section_subset_card(
            report,
            "Buses",
            "local storage and USB-adjacent hardware",
            BLUE,
        ))
}

fn services_panel(report: &MachineReport, control: &NodeControlState, width: f32) -> UiNode {
    UiNode::column("w-full gap-6")
        .child(
            UiNode::card("bg-panel border rounded-lg p-4 gap-3 h-28")
                .child(
                    UiNode::row("h-8 w-full items-center justify-between gap-3")
                        .child(UiNode::text("Host services").class("h-5 text-text truncate"))
                        .child(UiNode::badge(
                            &format!("{}/{} staged", control.service_count(), SERVICE_SETUP_TOTAL),
                            ACCENT,
                        )),
                )
                .child(UiNode::text(control.service_next_step()).class("h-5 text-muted truncate"))
                .child(
                    UiNode::progress_bar(
                        control.service_count() as f32 / SERVICE_SETUP_TOTAL as f32,
                        ACCENT,
                    )
                    .class("h-3 w-full"),
                ),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-4 gap-4", width)
                .class("w-full")
                .child(metric_node(
                    "Systemd",
                    if control.systemd_inventory {
                        "staged"
                    } else {
                        "inspect"
                    },
                    "unit discovery and service apply",
                    BLUE,
                ))
                .child(metric_node(
                    "Publishing",
                    if control.static_http {
                        "HTTP staged"
                    } else {
                        "not set"
                    },
                    "static HTTP, TLS, ACME, DNS",
                    ACCENT,
                ))
                .child(metric_node(
                    "Boot",
                    if control.ipxe_boot || control.tftp_boot {
                        "staged"
                    } else {
                        "not set"
                    },
                    "iPXE and TFTP provisioning",
                    AMBER,
                ))
                .child(metric_node(
                    "Mail",
                    if control.imap_service || control.smtp_service {
                        "staged"
                    } else {
                        "not set"
                    },
                    "IMAP and SMTP services",
                    BLUE,
                )),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Systemd Services",
                    "Inspect and stage host unit changes before signed apply.",
                    UiIcon::Terminal,
                    BLUE,
                    &[
                        control_button(
                            "Inspect systemd units",
                            "read current unit state and EdgeRun service files",
                            ACTION_SYSTEMD_INVENTORY,
                            if control.systemd_inventory {
                                "Staged"
                            } else {
                                "Inspect"
                            },
                        ),
                        control_value(
                            "unit apply mode",
                            "write systemd files only after signed approval",
                            "guarded",
                        ),
                        control_value(
                            "restart policy",
                            "show service restart before executing it",
                            "confirm",
                        ),
                    ],
                ))
                .child(control_card(
                    "Web Publishing",
                    "Static site hosting with DNS, TLS certs, and ACME automation.",
                    UiIcon::Network,
                    ACCENT,
                    &[
                        control_button(
                            "Set up static HTTP",
                            "serve content-addressed site bytes from this node",
                            ACTION_STATIC_HTTP,
                            if control.static_http {
                                "Staged"
                            } else {
                                "Set up"
                            },
                        ),
                        control_button(
                            "Create ACME certificates",
                            "request or renew TLS material under admission policy",
                            ACTION_ACME_CERTS,
                            if control.acme_certs {
                                "Staged"
                            } else {
                                "Create"
                            },
                        ),
                        control_button(
                            "Stage DNS zone",
                            "publish identity-routed DNS records for this node",
                            ACTION_DNS_ZONE,
                            if control.dns_zone { "Staged" } else { "Set up" },
                        ),
                    ],
                )),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Boot Services",
                    "Provision devices from identity-routed boot artifacts.",
                    UiIcon::Route,
                    AMBER,
                    &[
                        control_button(
                            "Enable iPXE",
                            "serve boot scripts and kernel arguments",
                            ACTION_IPXE_BOOT,
                            if control.ipxe_boot {
                                "Staged"
                            } else {
                                "Enable"
                            },
                        ),
                        control_button(
                            "Enable TFTP",
                            "serve boot artifacts on a local provisioning network",
                            ACTION_TFTP_BOOT,
                            if control.tftp_boot {
                                "Staged"
                            } else {
                                "Enable"
                            },
                        ),
                        control_value(
                            "boot artifacts",
                            "content-addressed objects from storage",
                            "planned",
                        ),
                    ],
                ))
                .child(control_card(
                    "Mail Services",
                    "User-owned mailbox ingress and submission endpoints.",
                    UiIcon::MessagePlus,
                    BLUE,
                    &[
                        control_button(
                            "Enable IMAP",
                            "serve local mailbox access under Trust Container policy",
                            ACTION_IMAP_SERVICE,
                            if control.imap_service {
                                "Staged"
                            } else {
                                "Enable"
                            },
                        ),
                        control_button(
                            "Enable SMTP",
                            "stage submission and delivery policy",
                            ACTION_SMTP_SERVICE,
                            if control.smtp_service {
                                "Staged"
                            } else {
                                "Enable"
                            },
                        ),
                        control_value(
                            "mail auth",
                            "DKIM/SPF/DMARC should bind to DNS policy",
                            "planned",
                        ),
                    ],
                )),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Proxy Routes",
                    "Expose approved local services through relay-mediated routes.",
                    UiIcon::Route,
                    ACCENT,
                    &[
                        control_button(
                            "Set up proxy",
                            "map public identity route to local service policy",
                            ACTION_PROXY_SERVICE,
                            if control.proxy_service {
                                "Staged"
                            } else {
                                "Set up"
                            },
                        ),
                        control_value(
                            "route authority",
                            "admission must sign accepted relay path",
                            "required",
                        ),
                        control_value(
                            "direct exposure",
                            "disabled unless explicit policy allows it",
                            "off",
                        ),
                    ],
                ))
                .child(control_card(
                    "Certificates & Passwords",
                    "Manage local cert material and service secrets behind Trust Container policy.",
                    UiIcon::Key,
                    AMBER,
                    &[
                        control_button(
                            "Manage passwords",
                            "stage secret records for service accounts",
                            ACTION_PASSWORD_VAULT,
                            if control.password_vault {
                                "Staged"
                            } else {
                                "Open"
                            },
                        ),
                        control_value(
                            "certificate storage",
                            "private keys should be device/notary-bound",
                            if control.acme_certs {
                                "staged"
                            } else {
                                "pending"
                            },
                        ),
                        control_value(
                            "secret export",
                            "never plaintext without signed unseal event",
                            "blocked",
                        ),
                    ],
                )),
        )
        .child(section_subset_card(
            report,
            "Network",
            "network interfaces and DNS visibility",
            BLUE,
        ))
        .child(section_subset_card(
            report,
            "Authority",
            "trust providers for certs and secrets",
            ACCENT,
        ))
}

fn trust_panel(report: &MachineReport, control: &NodeControlState, width: f32) -> UiNode {
    UiNode::column("w-full gap-6")
        .child(
            UiNode::card("bg-panel border rounded-lg p-4 gap-3 h-28")
                .child(
                    UiNode::row("h-8 w-full items-center justify-between gap-3")
                        .child(UiNode::text("Control plane state").class("h-5 text-text truncate"))
                        .child(UiNode::badge(&format!("{}/8 staged", control.setup_count()), ACCENT)),
                )
                .child(UiNode::text(&control.last_event).class("h-5 text-muted truncate"))
                .child(UiNode::progress_bar(control.setup_count() as f32 / 8.0, ACCENT).class("h-3 w-full")),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-4 gap-4", width)
                .class("w-full")
                .child(metric_node(
                    "Admission",
                    control.admission_label(),
                    control.admission_detail(),
                    if control.admission_mode == AdmissionMode::None { AMBER } else { ACCENT },
                ))
                .child(metric_node(
                    "Device Key",
                    if control.tpm_device_key { "TPM ready" } else { "not provisioned" },
                    &control.tpm_status,
                    if control.tpm_device_key { ACCENT } else { AMBER },
                ))
                .child(metric_node(
                    "Master Key",
                    if control.master_key_sealed { "sealed" } else { "locked" },
                    &control.sealed_key_status,
                    if control.master_key_sealed { ACCENT } else { BLUE },
                ))
                .child(metric_node("Proof Trail", "local", "audit events from this device", ACCENT)),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Next Steps",
                    "Dependency order before this device should admit real work.",
                    UiIcon::Route,
                    BLUE,
                    &[
                        control_value("current state", "local staged readiness", control.readiness_label()),
                        control_value("next action", "highest priority missing setup", control.next_step()),
                        control_value(
                            "signed execution",
                            "hardware actions run against local device backends",
                            if control.tpm_device_key || control.master_key_sealed {
                                "active"
                            } else {
                                "pending"
                            },
                        ),
                    ],
                ))
                .child(activity_card(control)),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Admission Authority",
                    "Choose the policy gate this device uses before work enters the network.",
                    UiIcon::Shield,
                    ACCENT,
                    &[
                        control_button(
                            "Use EdgeRun DAO admission",
                            "default public admission path",
                            ACTION_ADMISSION_DAO,
                            admission_button_label(control, AdmissionMode::EdgeRunDao),
                        ),
                        control_button(
                            "Create device admission",
                            "local policy for this machine",
                            ACTION_ADMISSION_DEVICE,
                            admission_button_label(control, AdmissionMode::Device),
                        ),
                        control_button(
                            "Attach owned admission",
                            "use a user or organization authority",
                            ACTION_ADMISSION_OWNED,
                            admission_button_label(control, AdmissionMode::Owned),
                        ),
                        control_value("route policy", "relay/channel assignment required", "strict"),
                    ],
                ))
                .child(control_card(
                    "Device Key",
                    "Bind this native node identity to hardware before admitting local roles.",
                    UiIcon::Key,
                    BLUE,
                    &[
                        control_button(
                            "Create TPM device key",
                            "derive node identity from hardware key",
                            ACTION_TPM_DEVICE_KEY,
                            if control.tpm_device_key { "Ready" } else { "Create" },
                        ),
                        control_button(
                            "Enroll YubiKey unlock",
                            "require security key for release",
                            ACTION_YUBIKEY_UNLOCK,
                            if control.yubikey_unlock { "Ready" } else { "Probe" },
                        ),
                        control_button(
                            "Enroll fingerprint unlock",
                            "local biometric gate when supported",
                            ACTION_FINGERPRINT_UNLOCK,
                            if control.fingerprint_unlock { "Ready" } else { "Enroll" },
                        ),
                        control_value(
                            "node identity",
                            "must match derive_node_id(public_key, role)",
                            if control.tpm_device_key { &control.tpm_status } else { "pending" },
                        ),
                        control_value("YubiKey", "PIV applet and attestation probe", &control.yubikey_status),
                        control_value("fingerprint", "hardware template enrollment", &control.fingerprint_status),
                    ],
                )),
        )
        .child(
            UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width)
                .class("w-full")
                .child(control_card(
                    "Node Signing Key",
                    "Generate and seal local node identity material before enabling service authority.",
                    UiIcon::Lock,
                    AMBER,
                    &[
                        control_button(
                            "Generate sealed node key",
                            "generate and seal device node key",
                            ACTION_SEAL_MASTER_KEY,
                            if control.master_key_sealed { "Sealed" } else { "Generate" },
                        ),
                        control_button(
                            "Test unseal",
                            "round-trip the sealed key envelope",
                            ACTION_TEST_UNSEAL,
                            if control.unseal_tested { "Passed" } else { "Run" },
                        ),
                        control_value("sealed key", "in-memory envelope from protocol seal crate", &control.sealed_key_status),
                        control_value("browser node handoff", "WASM node should ask host notary to release plaintext", "planned"),
                    ],
                ))
                .child(control_card(
                    "Proof Dashboard",
                    "Every admission, unseal, relay setup, and storage grant should leave evidence.",
                    UiIcon::Trust,
                    ACCENT,
                    &[
                        control_value(
                            "admission policy hash",
                            "content-addressed policy commitment",
                            if control.admission_mode == AdmissionMode::None { "unknown" } else { "staged" },
                        ),
                        control_value("delivery reports", "notary signs plaintext release events", "planned"),
                        control_value(
                            "relay/storage grants",
                            "show accepted route authority",
                            if control.network_storage { "staged" } else { "pending" },
                        ),
                    ],
                )),
        )
        .child(section_subset_card(report, "Authority", "detected local trust providers", ACCENT))
        .child(section_subset_card(report, "Input", "possible unlock factors on this device", AMBER))
}

fn metric_grid_layout(report: &MachineReport, width: f32) -> UiNode {
    UiNode::grid_auto_for_width(
        "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4",
        width,
    )
    .class("w-full")
    .child(metric_node("CPU", &report.cores, "logical cores", ACCENT))
    .child(metric_node("Memory", &report.memory, "physical RAM", BLUE))
    .child(metric_node(
        "Devices",
        &report.device_count.to_string(),
        "discovered entries",
        AMBER,
    ))
    .child(metric_node(
        "Providers",
        &report.capability_count.to_string(),
        "capability descriptors",
        ACCENT,
    ))
}

fn metric_node(title: &str, value: &str, detail: &str, accent: Color4) -> UiNode {
    UiNode::metric_card(title, value)
        .detail(detail)
        .accent(accent)
        .class("h-32")
}

#[derive(Clone, Copy)]
enum ControlAccessorySpec<'a> {
    Value(&'a str),
    Button(&'a str, u32),
}

#[derive(Clone, Copy)]
struct ControlSpec<'a> {
    label: &'a str,
    detail: &'a str,
    accessory: ControlAccessorySpec<'a>,
}

fn control_value<'a>(label: &'a str, detail: &'a str, value: &'a str) -> ControlSpec<'a> {
    ControlSpec {
        label,
        detail,
        accessory: ControlAccessorySpec::Value(value),
    }
}

fn admission_button_label(control: &NodeControlState, mode: AdmissionMode) -> &'static str {
    if control.admission_mode == mode {
        "Selected"
    } else {
        "Select"
    }
}

fn control_button<'a>(
    label: &'a str,
    detail: &'a str,
    id: u32,
    button_label: &'a str,
) -> ControlSpec<'a> {
    ControlSpec {
        label,
        detail,
        accessory: ControlAccessorySpec::Button(button_label, id),
    }
}

fn control_card(
    title: &str,
    detail: &str,
    icon: UiIcon,
    accent: Color4,
    rows: &[ControlSpec<'_>],
) -> UiNode {
    let height_units = 32 + rows.len().max(1) * 15;
    let class = format!("bg-panel border rounded-lg overflow-hidden p-4 gap-3 h-{height_units}");
    rows.iter().fold(
        UiNode::card(&class)
            .child(
                UiNode::row("h-12 w-full items-center gap-3")
                    .child(UiNode::icon(icon).accent(accent).class("size-6"))
                    .child(
                        UiNode::column("flex-1 gap-1")
                            .child(UiNode::text(title).class("h-5 text-text truncate"))
                            .child(UiNode::text(detail).class("h-5 text-muted truncate")),
                    ),
            )
            .child(UiNode::divider("w-full")),
        |node, row| node.child(control_row_node(row)),
    )
}

fn activity_card(control: &NodeControlState) -> UiNode {
    let visible = control.events.len().clamp(1, 5);
    let height_units = 30 + visible * 12;
    let class = format!("bg-panel border rounded-lg overflow-hidden p-4 gap-3 h-{height_units}");
    control.events.iter().take(visible).enumerate().fold(
        UiNode::card(&class)
            .child(
                UiNode::row("h-12 w-full items-center gap-3")
                    .child(
                        UiNode::icon(UiIcon::Activity)
                            .accent(ACCENT)
                            .class("size-6"),
                    )
                    .child(
                        UiNode::column("flex-1 gap-1")
                            .child(
                                UiNode::text("Local Event Timeline")
                                    .class("h-5 text-text truncate"),
                            )
                            .child(
                                UiNode::text(
                                    "Unsigned UI-stage events; protocol proofs come later",
                                )
                                .class("h-5 text-muted truncate"),
                            ),
                    ),
            )
            .child(UiNode::divider("w-full")),
        |node, (index, event)| {
            node.child(
                UiNode::list_row(event, "pending signed proof", 40_000 + index as u32)
                    .accent(ACCENT)
                    .class("h-9 rounded-sm bg-row"),
            )
        },
    )
}

fn list_card(
    title: &str,
    detail: &str,
    icon: UiIcon,
    accent: Color4,
    mut rows: Vec<String>,
    empty: &str,
) -> UiNode {
    if rows.is_empty() {
        rows.push(empty.to_string());
    }
    let visible = rows.len().clamp(1, 6);
    let height_units = 30 + visible * 12;
    let class = format!("bg-panel border rounded-lg overflow-hidden p-4 gap-3 h-{height_units}");
    rows.iter().take(visible).enumerate().fold(
        UiNode::card(&class)
            .child(
                UiNode::row("h-12 w-full items-center gap-3")
                    .child(UiNode::icon(icon).accent(accent).class("size-6"))
                    .child(
                        UiNode::column("flex-1 gap-1")
                            .child(UiNode::text(title).class("h-5 text-text truncate"))
                            .child(UiNode::text(detail).class("h-5 text-muted truncate")),
                    ),
            )
            .child(UiNode::divider("w-full")),
        |node, (index, row)| {
            node.child(
                UiNode::list_row(row, "", 41_000 + index as u32)
                    .accent(accent)
                    .class("h-9 rounded-sm bg-row"),
            )
        },
    )
}

fn control_row_node(row: &ControlSpec<'_>) -> UiNode {
    let node = UiNode::control_row(row.label)
        .detail(row.detail)
        .class("h-12 w-full rounded-sm bg-row");
    match row.accessory {
        ControlAccessorySpec::Value(value) => node.value_text(value),
        ControlAccessorySpec::Button(label, id) => {
            node.control_button(label, id, ButtonStyle::Secondary)
        }
    }
}

fn section_subset_card(
    report: &MachineReport,
    title: &'static str,
    detail: &str,
    accent: Color4,
) -> UiNode {
    report
        .sections
        .iter()
        .find(|section| section.title == title)
        .map(section_node)
        .unwrap_or_else(|| {
            let fallback = section(title, detail, accent, vec!["none discovered".to_string()]);
            section_node(&fallback)
        })
}

fn section_grid_layout(report: &MachineReport, width: f32) -> UiNode {
    report.sections.iter().fold(
        UiNode::grid_auto_for_width("grid grid-cols-1 lg:grid-cols-2 gap-4", width).class("w-full"),
        |node, section| node.child(section_node(section)),
    )
}

fn section_node(section: &ReportSection) -> UiNode {
    let visible_rows = section.rows.len().min(6);
    let height_units = 27 + visible_rows.max(1) * 12;
    let class = format!("bg-panel border rounded-lg overflow-hidden p-4 gap-3 h-{height_units}");
    let mut node = UiNode::card(&class)
        .child(
            UiNode::row("h-12 w-full items-center gap-3")
                .child(
                    UiNode::column("flex-1 gap-1")
                        .child(UiNode::text(section.title).class("h-5 text-text truncate"))
                        .child(UiNode::text(&section.detail).class("h-5 text-muted truncate")),
                )
                .child(UiNode::badge(
                    &section.rows.len().to_string(),
                    section.accent,
                )),
        )
        .child(UiNode::divider("w-full"));

    if section.rows.is_empty() {
        node = node.child(UiNode::text("none discovered").class("h-6 text-muted"));
    } else {
        for (index, row) in section.rows.iter().take(visible_rows).enumerate() {
            node = node.child(
                UiNode::list_row(row, "", 30_000 + index as u32)
                    .accent(section.accent)
                    .class("h-9 rounded-sm bg-row"),
            );
        }
    }
    node
}

fn refresh_label() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() % 86_400)
        .unwrap_or(0);
    let hour = seconds / 3600;
    let minute = (seconds % 3600) / 60;
    let second = seconds % 60;
    format!("{hour:02}:{minute:02}:{second:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_ui_core::gpu::{GpuScene, UiEvent, UiPainter, UiRect, UiRuntimeState};

    #[test]
    fn node_ui_layout_has_no_structural_issues_on_desktop_size() {
        let report = fixture_report();
        let control = NodeControlState::default();
        let layout = build_layout(
            &report,
            &control,
            ActivePanel::MachineReport,
            1180.0,
            760.0,
            0.0,
        );

        let issues = layout.layout_issues(UiRect::new(0.0, 0.0, 1180.0, 760.0));

        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn node_ui_layout_has_no_structural_issues_on_minimum_size() {
        let report = fixture_report();
        let control = NodeControlState::default();
        let layout = build_layout(
            &report,
            &control,
            ActivePanel::MachineReport,
            760.0,
            520.0,
            0.0,
        );

        let issues = layout.layout_issues(UiRect::new(0.0, 0.0, 760.0, 520.0));

        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn node_ui_all_panels_have_structural_layouts() {
        let report = fixture_report();
        let control = NodeControlState::default();
        for panel in [
            ActivePanel::MachineReport,
            ActivePanel::NodeInstances,
            ActivePanel::Storage,
            ActivePanel::Trust,
            ActivePanel::Services,
        ] {
            let layout = build_layout(&report, &control, panel, 1180.0, 760.0, 0.0);
            let issues = layout.layout_issues(UiRect::new(0.0, 0.0, 1180.0, 760.0));

            assert!(issues.is_empty(), "{panel:?}: {issues:?}");
        }
    }

    #[test]
    fn trust_panel_exposes_control_plane_actions() {
        let report = fixture_report();
        let control = NodeControlState::default();
        let layout = trust_panel(&report, &control, 932.0);
        let mut scene = GpuScene::new(BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            layout.render(&mut ui, UiRect::new(0.0, 0.0, 932.0, 1200.0));
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 21_002),
            "trust panel should expose device admission setup action: {:?}",
            scene.hits()
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 21_201),
            "trust panel should expose node key sealing action"
        );
    }

    #[test]
    fn control_actions_update_visible_state() {
        let report = fixture_report();
        let mut control = NodeControlState::default();

        assert!(control.apply_action(ACTION_ADMISSION_DEVICE));
        assert!(control.apply_action(ACTION_SEAL_MASTER_KEY));
        assert!(control.apply_action(ACTION_TEST_UNSEAL));
        assert_eq!(control.admission_label(), "device");
        assert_eq!(control.setup_count(), 3);
        assert_eq!(control.next_step(), "create TPM device key");
        assert!(
            control
                .events
                .first()
                .is_some_and(|event| event.starts_with("Sealed node key unseal passed: ")),
            "{:?}",
            control.events.first()
        );

        let layout = trust_panel(&report, &control, 932.0);
        let issues = layout.layout_issues(UiRect::new(0.0, 0.0, 932.0, 3400.0));

        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn local_authority_store_persists_public_node_state_without_seal_key() {
        let root = std::env::temp_dir().join(format!(
            "edgerun-node-ui-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = LocalAuthorityStore::new(root.clone());
        let sealed = generate_and_seal_node_key().expect("sealed node key");

        store.persist_node_key(&sealed).expect("persist node key");
        store
            .persist_status(TPM_STATUS_FILE, "test tpm status")
            .expect("persist tpm status");

        let loaded = store.load();
        assert!(loaded.master_key_sealed);
        assert!(loaded.tpm_device_key);
        assert_eq!(loaded.tpm_status, "test tpm status");
        assert_eq!(loaded.sealed_node_id, Some(sealed.node_id));
        assert_eq!(
            loaded.sealed_node_key.as_deref(),
            Some(sealed.envelope.as_slice())
        );
        assert!(loaded.seal_key.is_none(), "seal key must not be persisted");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn services_panel_exposes_host_control_actions() {
        let report = fixture_report();
        let mut control = NodeControlState::default();

        assert!(control.apply_action(ACTION_SYSTEMD_INVENTORY));
        assert!(control.apply_action(ACTION_STATIC_HTTP));
        assert!(control.apply_action(ACTION_ACME_CERTS));
        assert_eq!(control.service_count(), 3);
        assert_eq!(control.service_next_step(), "stage DNS authority");
        assert_eq!(control.service_apply_status(), "blocked");
        assert!(
            control
                .service_blockers()
                .iter()
                .any(|blocker| blocker.contains("admission"))
        );
        assert!(
            control
                .service_apply_queue()
                .iter()
                .any(|change| change.starts_with("http:"))
        );

        let layout = services_panel(&report, &control, 932.0);
        let mut scene = GpuScene::new(BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            layout.render(&mut ui, UiRect::new(0.0, 0.0, 932.0, 3600.0));
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == ACTION_DNS_ZONE),
            "services panel should expose DNS setup action"
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == ACTION_PASSWORD_VAULT),
            "services panel should expose password management action"
        );
    }

    #[test]
    fn node_sidebar_nav_items_emit_runtime_actions() {
        let report = fixture_report();
        let control = NodeControlState::default();
        let layout = build_layout(
            &report,
            &control,
            ActivePanel::MachineReport,
            1180.0,
            760.0,
            0.0,
        );
        let mut scene = GpuScene::new(BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            layout.render(&mut ui, UiRect::new(0.0, 0.0, 1180.0, 760.0));
        }

        let hit = scene
            .hits()
            .iter()
            .copied()
            .find(|hit| hit.kind == HitKind::MenuItem && hit.id == NAV_STORAGE_ID)
            .expect("storage nav item should render as a menu hit");

        let mut runtime = UiRuntimeState::default();
        assert_eq!(
            runtime.handle_event(
                &scene,
                UiEvent::PointerDown {
                    x: hit.x + 4.0,
                    y: hit.y + 4.0,
                },
            ),
            UiAction::Activated(hit),
        );
        assert_eq!(
            nav_panel_for_id(hit.id),
            Some(ActivePanel::Storage),
            "node event handler must be able to route the emitted menu id",
        );
    }

    fn fixture_report() -> MachineReport {
        MachineReport {
            refreshed_at: "12:34:56".to_string(),
            platform: "linux",
            cores: "16".to_string(),
            memory: "64.0 GB".to_string(),
            capability_count: 42,
            device_count: 24,
            sections: vec![
                section(
                    "Compute",
                    "GPU and accelerator devices",
                    ACCENT,
                    vec![
                        "GPU 0000:c1:00.0 (1002:15bf)".to_string(),
                        "Linux NPU accel0".to_string(),
                        "AMD XDNA NPU".to_string(),
                    ],
                ),
                section(
                    "Network",
                    "WiFi, Bluetooth, NFC, and CEC",
                    BLUE,
                    vec![
                        "wlan0 connected".to_string(),
                        "bluetooth hci0".to_string(),
                        "nfc none".to_string(),
                    ],
                ),
                section("Power", "Battery and power supply state", AMBER, Vec::new()),
            ],
        }
    }
}
