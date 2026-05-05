use std::str::FromStr;
use std::sync::Arc;

use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
use edgerun_crypto::p256::ecdsa::SigningKey;
use edgerun_hardware_signing::{
    HardwareMeshSigner, MeshSigner, NodeID, TpmHardwareKeyAdapter, YubiKeyHardwareKeyAdapter,
};
use edgerun_tpm::{LinuxTpmSigningKey, TpmHandle};
use edgerun_yubikey::{LinuxPcscYubiKey, YubiKeyPivSlot};

use crate::config::{NodeConfig, SignerConfig};
use crate::init_cmd::detect_yubikey_device;

/// A software signer that is Send + Sync.
pub struct SyncSoftwareSigner {
    node_id: NodeID,
    key: std::sync::Mutex<SigningKey>,
}

impl SyncSoftwareSigner {
    pub fn new(key: SigningKey) -> Self {
        let vk = key.verifying_key();
        let encoded = vk.to_encoded_point(false);
        let mut node_bytes = [0u8; 64];
        node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
        Self {
            node_id: NodeID(node_bytes),
            key: std::sync::Mutex::new(key),
        }
    }
}

impl MeshSigner for SyncSoftwareSigner {
    fn node_id(&self) -> NodeID {
        self.node_id
    }

    fn sign_digest(
        &self,
        digest: &[u8; 32],
    ) -> Result<[u8; 64], edgerun_hardware_signing::HardwareSigningError> {
        let key = self
            .key
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let sig: edgerun_crypto::p256::ecdsa::Signature = key
            .sign_prehash(digest)
            .map_err(|e| edgerun_hardware_signing::HardwareSigningError::Provider(e.to_string()))?;
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&sig.to_bytes());
        Ok(bytes)
    }

    fn sign_message_var(
        &self,
        message: &[u8],
    ) -> Result<[u8; 64], edgerun_hardware_signing::HardwareSigningError> {
        let key = self
            .key
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let sig: edgerun_crypto::p256::ecdsa::Signature = key
            .sign_prehash(message)
            .map_err(|e| edgerun_hardware_signing::HardwareSigningError::Provider(e.to_string()))?;
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&sig.to_bytes());
        Ok(bytes)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub fn load_signer_from_config(config: &NodeConfig) -> Arc<dyn MeshSigner + Send + Sync> {
    let Some(signer_config) = &config.signer else {
        eprintln!("error: no signer configured. Run `edgerund init` first.");
        std::process::exit(1);
    };

    match signer_config.signer_type.as_str() {
        "software" => {
            let Some(key_hex) = &signer_config.private_key_hex else {
                eprintln!("error: software signer configured but private_key_hex is missing.");
                std::process::exit(1);
            };
            let signing_key = parse_signing_key_hex(key_hex);
            Arc::new(SyncSoftwareSigner::new(signing_key))
        }
        "tpm" => {
            let handle_hex = signer_config
                .handle
                .as_ref()
                .expect("TPM signer requires handle");
            let handle =
                edgerun_encoding::hex::parse_hex_int::<u32>(handle_hex.trim_start_matches("0x"))
                    .unwrap_or_else(|| {
                        eprintln!("error: invalid TPM handle '{}'", handle_hex);
                        std::process::exit(1);
                    });

            edgerun_log::info!("using TPM signer: handle=0x{:08X}", handle);

            let tpm_key = LinuxTpmSigningKey::new("/dev/tpmrm0", TpmHandle(handle));

            let adapter = TpmHardwareKeyAdapter::new(tpm_key);
            let mesh_signer = HardwareMeshSigner::new(adapter).unwrap_or_else(|e| {
                eprintln!("error: failed to initialize TPM signer: {}", e);
                std::process::exit(1);
            });

            Arc::new(mesh_signer)
        }
        "yubikey" => {
            let slot_str = signer_config
                .handle
                .as_ref()
                .expect("YubiKey signer requires handle");
            let slot = match slot_str.as_str() {
                "9a" => YubiKeyPivSlot::Authentication,
                "9c" => YubiKeyPivSlot::Signature,
                "9d" => YubiKeyPivSlot::KeyManagement,
                "9e" => YubiKeyPivSlot::CardAuthentication,
                _ => {
                    eprintln!("error: unsupported YubiKey slot: {}", slot_str);
                    std::process::exit(1);
                }
            };

            let device = detect_yubikey_device().unwrap_or_else(|e| {
                eprintln!("error: no YubiKey device found: {}", e);
                std::process::exit(1);
            });

            edgerun_log::info!(
                "using YubiKey signer: bus={:03}, device={:03}, slot={}",
                device.bus,
                device.device,
                slot_str
            );

            let yubikey = LinuxPcscYubiKey::new(device, slot);
            let adapter = YubiKeyHardwareKeyAdapter::new(yubikey);
            let mesh_signer = HardwareMeshSigner::new(adapter).unwrap_or_else(|e| {
                eprintln!("error: failed to initialize YubiKey signer: {}", e);
                std::process::exit(1);
            });

            Arc::new(mesh_signer)
        }
        "encrypted" => {
            let key_path = signer_config
                .encrypted_key_path
                .as_ref()
                .expect("encrypted signer requires encrypted_key_path");
            let passphrase_env = signer_config
                .passphrase_env
                .as_deref()
                .unwrap_or("EDGERUN_KEY_PASSPHRASE");

            let passphrase = std::env::var(passphrase_env).unwrap_or_else(|_| {
                eprintln!(
                    "error: encrypted signer requires the passphrase env var '{}' to be set.",
                    passphrase_env
                );
                eprintln!("Set it with: export {}='your-passphrase'", passphrase_env);
                std::process::exit(1);
            });

            let encrypted_data = std::fs::read(key_path).unwrap_or_else(|e| {
                eprintln!(
                    "error: failed to read encrypted key file '{}': {}",
                    key_path, e
                );
                std::process::exit(1);
            });

            let signing_key = edgerun_crypto::decrypt_signing_key(&encrypted_data, &passphrase)
                .unwrap_or_else(|e| {
                    eprintln!(
                    "error: failed to decrypt key with passphrase from '{}' (wrong passphrase?)",
                    passphrase_env
                );
                    std::process::exit(1);
                });

            Arc::new(SyncSoftwareSigner::new(signing_key))
        }
        "provisioned" => {
            let passphrase_env = signer_config
                .passphrase_env
                .as_deref()
                .unwrap_or("EDGERUN_KEY_PASSPHRASE");

            let passphrase = std::env::var(passphrase_env).unwrap_or_else(|_| {
                eprintln!(
                    "error: provisioned signer requires passphrase env var '{}' to be set.",
                    passphrase_env
                );
                eprintln!("Set it with: export {}='your-passphrase'", passphrase_env);
                std::process::exit(1);
            });

            if passphrase.len() < 8 {
                eprintln!("error: passphrase must be at least 8 characters");
                std::process::exit(1);
            }

            let signing_key =
                derive_signing_key_from_passphrase(&passphrase, &signer_config.public_key_hex)
                    .unwrap_or_else(|e| {
                        eprintln!("error: failed to derive signing key: {}", e);
                        std::process::exit(1);
                    });

            Arc::new(SyncSoftwareSigner::new(signing_key))
        }
        other => {
            eprintln!("error: unknown signer type: {}", other);
            std::process::exit(1);
        }
    }
}

const PBKDF2_ITERATIONS: u32 = 100_000;

fn derive_signing_key_from_passphrase(
    passphrase: &str,
    public_key_hex: &str,
) -> Result<SigningKey, String> {
    use edgerun_crypto::pbkdf2::pbkdf2_hmac_array;
    use edgerun_crypto::sha2::Sha256;

    let salt = format!("edgerun:provisioned:{}", public_key_hex);
    let derived: [u8; 32] =
        pbkdf2_hmac_array::<Sha256, 32>(passphrase.as_bytes(), salt.as_bytes(), PBKDF2_ITERATIONS);
    SigningKey::from_bytes(&derived.into()).map_err(|e| e.to_string())
}

pub fn parse_signing_key_hex(key_hex: &str) -> SigningKey {
    let hex_str = key_hex.trim();
    let bytes = edgerun_core::util::hex_to_bytes(hex_str).unwrap_or_else(|e| {
        eprintln!("error: invalid key hex: {}", e);
        std::process::exit(1);
    });
    let bytes: [u8; 32] = bytes.try_into().unwrap_or_else(|_| {
        eprintln!("error: key must be 32 bytes (64 hex chars)");
        std::process::exit(1);
    });
    SigningKey::from_bytes(&bytes.into()).unwrap_or_else(|e| {
        eprintln!("error: invalid key: {}", e);
        std::process::exit(1);
    })
}
