use std::str::FromStr;
use std::sync::Arc;

use edgerun_hardware_signing::{
    HardwareMeshSigner, MeshSigner, NodeID, TpmHardwareKeyAdapter, YubiKeyHardwareKeyAdapter,
};
use edgerun_protocols::keygen::{
    node_id_from_signing_key, node_signing_key_from_bytes, NodeSigningKey,
};
use edgerun_protocols::seal::{unseal_node_signing_key, SealKey};
use edgerun_protocols::sign::{ProtocolSigner, SignableProtocolFamily};
use edgerun_protocols::sign_p256::P256ProtocolSigner;
use edgerun_tpm::{LinuxTpmSigningKey, TpmHandle};
use edgerun_yubikey::{LinuxPcscYubiKey, YubiKeyPivSlot};

use crate::config::{NodeConfig, SignerConfig};
use crate::init_cmd::detect_yubikey_device;

/// A software signer that is Send + Sync.
pub struct SyncSoftwareSigner {
    node_id: NodeID,
    signer: std::sync::Mutex<P256ProtocolSigner>,
}

impl SyncSoftwareSigner {
    pub fn new(key: NodeSigningKey) -> Self {
        let node_bytes = node_id_from_signing_key(&key);
        Self {
            node_id: NodeID(node_bytes),
            signer: std::sync::Mutex::new(P256ProtocolSigner::new(key)),
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
        let signer = self
            .signer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let sig = signer
            .sign_signature_input(SignableProtocolFamily::EventEnvelope, digest)
            .map_err(|e| {
                edgerun_hardware_signing::HardwareSigningError::Provider(format!("{e:?}"))
            })?;
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&sig);
        Ok(bytes)
    }

    fn sign_message_var(
        &self,
        message: &[u8],
    ) -> Result<[u8; 64], edgerun_hardware_signing::HardwareSigningError> {
        let signer = self
            .signer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let sig = signer
            .sign_signature_input(SignableProtocolFamily::EventEnvelope, message)
            .map_err(|e| {
                edgerun_hardware_signing::HardwareSigningError::Provider(format!("{e:?}"))
            })?;
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&sig);
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

            crate::node_info!("using TPM signer: handle=0x{:08X}", handle);

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

            crate::node_info!(
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
            let seal_key = load_seal_key(signer_config);

            let encrypted_data = std::fs::read(key_path).unwrap_or_else(|e| {
                eprintln!(
                    "error: failed to read encrypted key file '{}': {}",
                    key_path, e
                );
                std::process::exit(1);
            });

            let signing_key =
                unseal_node_signing_key(&encrypted_data, &seal_key).unwrap_or_else(|e| {
                    eprintln!("error: failed to unseal node signing key: {:?}", e);
                    std::process::exit(1);
                });

            Arc::new(SyncSoftwareSigner::new(signing_key))
        }
        "provisioned" => {
            let Some(key_hex) = &signer_config.private_key_hex else {
                eprintln!("error: provisioned signer configured but private_key_hex is missing.");
                std::process::exit(1);
            };
            let signing_key = parse_signing_key_hex(key_hex);
            Arc::new(SyncSoftwareSigner::new(signing_key))
        }
        other => {
            eprintln!("error: unknown signer type: {}", other);
            std::process::exit(1);
        }
    }
}

fn load_seal_key(signer_config: &SignerConfig) -> SealKey {
    if let Some(key_hex) = &signer_config.seal_key_hex {
        return parse_seal_key_hex(key_hex);
    }

    let key_env = signer_config
        .seal_key_env
        .as_deref()
        .unwrap_or("EDGERUN_SEAL_KEY_HEX");
    let key_hex = std::env::var(key_env).unwrap_or_else(|_| {
        eprintln!(
            "error: encrypted signer requires seal key env var '{}' to be set.",
            key_env
        );
        std::process::exit(1);
    });
    parse_seal_key_hex(&key_hex)
}

fn parse_seal_key_hex(key_hex: &str) -> SealKey {
    let bytes = edgerun_protocols::core_protocol::util::hex_to_bytes(key_hex.trim())
        .unwrap_or_else(|e| {
            eprintln!("error: invalid seal key hex: {}", e);
            std::process::exit(1);
        });
    let bytes: [u8; 32] = bytes.try_into().unwrap_or_else(|_| {
        eprintln!("error: seal key must be 32 bytes (64 hex chars)");
        std::process::exit(1);
    });
    SealKey::from_bytes(bytes)
}

pub fn parse_signing_key_hex(key_hex: &str) -> NodeSigningKey {
    let hex_str = key_hex.trim();
    let bytes = edgerun_protocols::core_protocol::util::hex_to_bytes(hex_str).unwrap_or_else(|e| {
        eprintln!("error: invalid key hex: {}", e);
        std::process::exit(1);
    });
    let bytes: [u8; 32] = bytes.try_into().unwrap_or_else(|_| {
        eprintln!("error: key must be 32 bytes (64 hex chars)");
        std::process::exit(1);
    });
    node_signing_key_from_bytes(bytes).unwrap_or_else(|e| {
        eprintln!("error: invalid key: {:?}", e);
        std::process::exit(1);
    })
}
