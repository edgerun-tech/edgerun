#[derive(Clone, Debug)]
pub struct NodeConfig {
    pub stream_id: String,
    pub name: Option<String>,
    pub controllers: Vec<String>,
    pub trust_nodes: Vec<String>,
    pub allowed_peers: Vec<String>,
    pub bootstrap_peers: Vec<String>,
    pub signer: Option<SignerConfig>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SignerState {
    Provisioning,
    Locked,
    Active,
}

#[derive(Clone, Debug)]
pub struct SignerConfig {
    pub signer_type: String,
    pub public_key_hex: String,
    pub private_key_hex: Option<String>,
    pub handle: Option<String>,
    pub slot: Option<String>,
    pub encrypted_key_path: Option<String>,
    pub seal_key_hex: Option<String>,
    pub seal_key_env: Option<String>,
    pub state: Option<SignerState>,
    pub pairing_pin: Option<String>,
}

pub fn parse_config(yaml: &str) -> Result<NodeConfig, String> {
    let mut config = NodeConfig {
        stream_id: String::new(),
        name: None,
        controllers: Vec::new(),
        trust_nodes: Vec::new(),
        allowed_peers: Vec::new(),
        bootstrap_peers: Vec::new(),
        signer: None,
    };

    let mut in_signer = false;
    let mut signer_type = String::new();
    let mut public_key_hex = String::new();
    let mut private_key_hex = Option::<String>::None;
    let mut handle = Option::<String>::None;
    let mut slot = Option::<String>::None;
    let mut encrypted_key_path = Option::<String>::None;
    let mut seal_key_hex = Option::<String>::None;
    let mut seal_key_env = Option::<String>::None;
    let mut signer_state = Option::<SignerState>::None;
    let mut pairing_pin = Option::<String>::None;

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with("signer:") {
            in_signer = true;
            continue;
        }

        if in_signer && !line.starts_with(' ') && !line.starts_with('\t') {
            in_signer = false;
        }

        if in_signer {
            let content = trimmed.trim_start();
            if let Some(val) = parse_kv(content) {
                match val.0.as_str() {
                    "type" => signer_type = unquote(&val.1),
                    "public_key_hex" => public_key_hex = unquote(&val.1),
                    "private_key_hex" => private_key_hex = Some(unquote(&val.1)),
                    "handle" => handle = Some(unquote(&val.1)),
                    "slot" => slot = Some(unquote(&val.1)),
                    "encrypted_key_path" => encrypted_key_path = Some(unquote(&val.1)),
                    "seal_key_hex" => seal_key_hex = Some(unquote(&val.1)),
                    "seal_key_env" => seal_key_env = Some(unquote(&val.1)),
                    "state" => {
                        signer_state = match val.1.as_str() {
                            "provisioning" => Some(SignerState::Provisioning),
                            "locked" => Some(SignerState::Locked),
                            "active" => Some(SignerState::Active),
                            _ => None,
                        };
                    }
                    "pairing_pin" => pairing_pin = Some(unquote(&val.1)),
                    _ => {}
                }
            }
        } else {
            if let Some((key, val)) = parse_kv(trimmed) {
                match key.as_str() {
                    "stream_id" => config.stream_id = unquote(&val),
                    "name" => config.name = Some(unquote(&val)),
                    "controllers" => config.controllers = parse_list(&val),
                    "trust_nodes" => config.trust_nodes = parse_list(&val),
                    "allowed_peers" => config.allowed_peers = parse_list(&val),
                    "bootstrap_peers" => config.bootstrap_peers = parse_list(&val),
                    _ => {}
                }
            }
        }
    }

    if !signer_type.is_empty() || !public_key_hex.is_empty() {
        config.signer = Some(SignerConfig {
            signer_type: if signer_type.is_empty() {
                "unknown".into()
            } else {
                signer_type
            },
            public_key_hex,
            private_key_hex,
            handle,
            slot,
            encrypted_key_path,
            seal_key_hex,
            seal_key_env,
            state: signer_state,
            pairing_pin,
        });
    }

    if config.stream_id.is_empty() {
        return Err("missing required field: stream_id".into());
    }
    Ok(config)
}

pub fn parse_kv(line: &str) -> Option<(String, String)> {
    edgerun_encoding::kv::parse_kv_colon(line)
}

pub fn parse_list(val: &str) -> Vec<String> {
    edgerun_encoding::kv::parse_bracket_list(val)
}

pub fn unquote(s: &str) -> String {
    edgerun_encoding::kv::unquote(s)
}

/// Parsed bootstrap peer configuration.
#[derive(Clone, Debug)]
pub struct BootstrapPeer {
    pub addr: String,
    pub node_id_hex: String,
}

pub fn parse_bootstrap_peers(entries: &[String]) -> Vec<BootstrapPeer> {
    entries
        .iter()
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.splitn(2, '@').collect();
            if parts.len() == 2 {
                Some(BootstrapPeer {
                    addr: parts[0].to_string(),
                    node_id_hex: parts[1].to_string(),
                })
            } else {
                crate::node_warn!("invalid bootstrap peer format (expected host:port@node_id_hex)");
                None
            }
        })
        .collect()
}

pub fn extract_private_key_bytes(config: &NodeConfig) -> Vec<u8> {
    if let Some(ref signer) = config.signer {
        if signer.signer_type == "software" {
            if let Some(ref hex_str) = signer.private_key_hex {
                return edgerun_protocols::core_protocol::util::hex_to_bytes(hex_str.trim())
                    .unwrap_or_else(|_| {
                        eprintln!("error: invalid private key hex");
                        std::process::exit(1);
                    });
            }
        } else if signer.signer_type == "tpm" || signer.signer_type == "yubikey" {
            return Vec::new();
        }
    }
    eprintln!("error: no software signer with private_key_hex found in config");
    std::process::exit(1);
}
