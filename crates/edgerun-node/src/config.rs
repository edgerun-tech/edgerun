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

#[derive(Clone, Debug)]
pub struct SignerConfig {
    pub signer_type: String,
    pub public_key_hex: String,
    pub private_key_hex: Option<String>,
    pub handle: Option<String>,
    pub slot: Option<String>,
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

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }

        // Detect signer section
        if trimmed.starts_with("signer:") {
            in_signer = true;
            continue;
        }

        // Other top-level sections end signer parsing
        // A top-level key has no leading whitespace (check original line, not trimmed)
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
            signer_type: if signer_type.is_empty() { "unknown".into() } else { signer_type },
            public_key_hex,
            private_key_hex,
            handle,
            slot,
        });
    }

    if config.stream_id.is_empty() {
        return Err("missing required field: stream_id".into());
    }
    Ok(config)
}

pub fn parse_kv(line: &str) -> Option<(String, String)> {
    let colon = line.find(':')?;
    let key = line[..colon].trim().to_string();
    let val = line[colon + 1..].trim().to_string();
    Some((key, val))
}

pub fn parse_list(val: &str) -> Vec<String> {
    let val = val.trim();
    if val == "[]" || val.is_empty() { return Vec::new(); }
    // Handle [item1, item2] format
    if val.starts_with('[') && val.ends_with(']') {
        let inner = &val[1..val.len() - 1];
        if inner.trim().is_empty() { return Vec::new(); }
        return inner.split(',').map(|s| unquote(s.trim())).collect();
    }
    // Handled elsewhere
    Vec::new()
}

pub fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 {
        let bytes = s.as_bytes();
        if (bytes[0] == b'"' && bytes[s.len() - 1] == b'"') ||
           (bytes[0] == 39 && bytes[s.len() - 1] == 39) {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

/// Parsed bootstrap peer configuration.
#[derive(Clone, Debug)]
pub struct BootstrapPeer {
    pub addr: String,
    pub node_id_hex: String,
}

pub fn parse_bootstrap_peers(entries: &[String]) -> Vec<BootstrapPeer> {
    entries.iter().filter_map(|entry| {
        // Format: "host:port@node_id_hex"
        let parts: Vec<&str> = entry.splitn(2, '@').collect();
        if parts.len() == 2 {
            Some(BootstrapPeer {
                addr: parts[0].to_string(),
                node_id_hex: parts[1].to_string(),
            })
        } else {
            edgerun_log::warn!("invalid bootstrap peer format (expected host:port@node_id_hex)");
            None
        }
    }).collect()
}

pub fn extract_private_key_bytes(config: &NodeConfig) -> Vec<u8> {
    if let Some(ref signer) = config.signer {
        if signer.signer_type == "software" {
            if let Some(ref hex_str) = signer.private_key_hex {
                return edgerun_core::util::hex_to_bytes(hex_str.trim()).unwrap_or_else(|_| {
                    eprintln!("error: invalid private key hex");
                    std::process::exit(1);
                });
            }
        } else if signer.signer_type == "tpm" || signer.signer_type == "yubikey" {
            // Hardware signers don't expose private keys.
            // For blob encryption, derive a key from the public key or use a separate mechanism.
            // Return empty vec for now -- the storage layer should handle this gracefully.
            return Vec::new();
        }
    }
    eprintln!("error: no software signer with private_key_hex found in config");
    std::process::exit(1);
}
