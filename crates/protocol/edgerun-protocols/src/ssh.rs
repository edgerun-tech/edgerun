//! SSH protocol framing and negotiation helpers.

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

pub const CLIENT_IDENTIFICATION: &str = "SSH-2.0-edgerun-ssh_0.1";
pub const SSH_MSG_IGNORE: u8 = 2;
pub const SSH_MSG_DEBUG: u8 = 4;
pub const SSH_MSG_SERVICE_REQUEST: u8 = 5;
pub const SSH_MSG_SERVICE_ACCEPT: u8 = 6;
pub const SSH_MSG_EXT_INFO: u8 = 7;
pub const SSH_MSG_KEXINIT: u8 = 20;
pub const SSH_MSG_NEWKEYS: u8 = 21;
pub const SSH_MSG_KEX_ECDH_INIT: u8 = 30;
pub const SSH_MSG_KEX_ECDH_REPLY: u8 = 31;
pub const SSH_MSG_USERAUTH_REQUEST: u8 = 50;
pub const SSH_MSG_USERAUTH_FAILURE: u8 = 51;
pub const SSH_MSG_USERAUTH_SUCCESS: u8 = 52;
pub const SSH_MSG_GLOBAL_REQUEST: u8 = 80;
pub const SSH_MSG_REQUEST_SUCCESS: u8 = 81;
pub const SSH_MSG_REQUEST_FAILURE: u8 = 82;
pub const SSH_MSG_CHANNEL_OPEN: u8 = 90;
pub const SSH_MSG_CHANNEL_OPEN_CONFIRMATION: u8 = 91;
pub const SSH_MSG_CHANNEL_OPEN_FAILURE: u8 = 92;
pub const SSH_MSG_CHANNEL_WINDOW_ADJUST: u8 = 93;
pub const SSH_MSG_CHANNEL_DATA: u8 = 94;
pub const SSH_MSG_CHANNEL_EXTENDED_DATA: u8 = 95;
pub const SSH_MSG_CHANNEL_EOF: u8 = 96;
pub const SSH_MSG_CHANNEL_CLOSE: u8 = 97;
pub const SSH_MSG_CHANNEL_REQUEST: u8 = 98;
pub const SSH_MSG_CHANNEL_SUCCESS: u8 = 99;
pub const SSH_MSG_CHANNEL_FAILURE: u8 = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SshError {
    BadTarget,
    Io(String),
    BadIdentification,
    BadPacket,
    UnsupportedMessage(u8),
    UnsupportedAlgorithm,
    AuthenticationFailed(String),
    BadKey,
    BadSignature,
    MacMismatch,
}

impl core::fmt::Display for SshError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadTarget => f.write_str("bad ssh target"),
            Self::Io(message) => f.write_str(message),
            Self::BadIdentification => f.write_str("target did not send an SSH identification"),
            Self::BadPacket => f.write_str("bad ssh packet"),
            Self::UnsupportedMessage(message) => write!(f, "unsupported ssh message: {message}"),
            Self::UnsupportedAlgorithm => f.write_str("unsupported ssh algorithm negotiation"),
            Self::AuthenticationFailed(message) => {
                write!(f, "ssh authentication failed: {message}")
            }
            Self::BadKey => f.write_str("bad ssh key"),
            Self::BadSignature => f.write_str("bad ssh signature"),
            Self::MacMismatch => f.write_str("ssh packet mac mismatch"),
        }
    }
}

impl core::error::Error for SshError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshTarget {
    pub user: String,
    pub host: String,
    pub port: u16,
}

impl SshTarget {
    pub fn parse(input: &str) -> Result<Self, SshError> {
        let (user, rest) = input.split_once('@').ok_or(SshError::BadTarget)?;
        if user.is_empty() || rest.is_empty() {
            return Err(SshError::BadTarget);
        }
        let (host, port) = if let Some((host, port)) = rest.rsplit_once(':') {
            if rest.matches(':').count() == 1 {
                let port = port.parse::<u16>().map_err(|_| SshError::BadTarget)?;
                (host.to_string(), port)
            } else {
                (rest.to_string(), 22)
            }
        } else {
            (rest.to_string(), 22)
        };
        if host.is_empty() {
            return Err(SshError::BadTarget);
        }
        Ok(Self {
            user: user.to_string(),
            host,
            port,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshIdentification {
    pub server: String,
    pub client: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshKexInit {
    pub cookie: [u8; 16],
    pub kex_algorithms: Vec<String>,
    pub server_host_key_algorithms: Vec<String>,
    pub encryption_algorithms_client_to_server: Vec<String>,
    pub encryption_algorithms_server_to_client: Vec<String>,
    pub mac_algorithms_client_to_server: Vec<String>,
    pub mac_algorithms_server_to_client: Vec<String>,
    pub compression_algorithms_client_to_server: Vec<String>,
    pub compression_algorithms_server_to_client: Vec<String>,
    pub languages_client_to_server: Vec<String>,
    pub languages_server_to_client: Vec<String>,
    pub first_kex_packet_follows: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshProbe {
    pub identification: SshIdentification,
    pub server_kex: SshKexInit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshAlgorithmSelection {
    pub kex: String,
    pub host_key: String,
    pub encryption_client_to_server: String,
    pub encryption_server_to_client: String,
    pub mac_client_to_server: String,
    pub mac_server_to_client: String,
    pub compression_client_to_server: String,
    pub compression_server_to_client: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshExecResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_status: Option<u32>,
}

pub trait SshExecTransport {
    fn exec(&mut self, command: &str) -> Result<Vec<u8>, SshError>;
}

pub fn encode_kexinit(kex: &SshKexInit) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(SSH_MSG_KEXINIT);
    payload.extend_from_slice(&kex.cookie);
    write_name_list(&mut payload, &kex.kex_algorithms);
    write_name_list(&mut payload, &kex.server_host_key_algorithms);
    write_name_list(&mut payload, &kex.encryption_algorithms_client_to_server);
    write_name_list(&mut payload, &kex.encryption_algorithms_server_to_client);
    write_name_list(&mut payload, &kex.mac_algorithms_client_to_server);
    write_name_list(&mut payload, &kex.mac_algorithms_server_to_client);
    write_name_list(&mut payload, &kex.compression_algorithms_client_to_server);
    write_name_list(&mut payload, &kex.compression_algorithms_server_to_client);
    write_name_list(&mut payload, &kex.languages_client_to_server);
    write_name_list(&mut payload, &kex.languages_server_to_client);
    payload.push(u8::from(kex.first_kex_packet_follows));
    payload.extend_from_slice(&0u32.to_be_bytes());
    payload
}

pub fn parse_kexinit(payload: &[u8]) -> Result<SshKexInit, SshError> {
    if payload.len() < 17 {
        return Err(SshError::BadPacket);
    }
    if payload[0] != SSH_MSG_KEXINIT {
        return Err(SshError::UnsupportedMessage(payload[0]));
    }
    let mut cursor = 1;
    let mut cookie = [0u8; 16];
    cookie.copy_from_slice(&payload[cursor..cursor + 16]);
    cursor += 16;
    let kex_algorithms = read_name_list(payload, &mut cursor)?;
    let server_host_key_algorithms = read_name_list(payload, &mut cursor)?;
    let encryption_algorithms_client_to_server = read_name_list(payload, &mut cursor)?;
    let encryption_algorithms_server_to_client = read_name_list(payload, &mut cursor)?;
    let mac_algorithms_client_to_server = read_name_list(payload, &mut cursor)?;
    let mac_algorithms_server_to_client = read_name_list(payload, &mut cursor)?;
    let compression_algorithms_client_to_server = read_name_list(payload, &mut cursor)?;
    let compression_algorithms_server_to_client = read_name_list(payload, &mut cursor)?;
    let languages_client_to_server = read_name_list(payload, &mut cursor)?;
    let languages_server_to_client = read_name_list(payload, &mut cursor)?;
    if cursor + 5 > payload.len() {
        return Err(SshError::BadPacket);
    }
    let first_kex_packet_follows = payload[cursor] != 0;
    Ok(SshKexInit {
        cookie,
        kex_algorithms,
        server_host_key_algorithms,
        encryption_algorithms_client_to_server,
        encryption_algorithms_server_to_client,
        mac_algorithms_client_to_server,
        mac_algorithms_server_to_client,
        compression_algorithms_client_to_server,
        compression_algorithms_server_to_client,
        languages_client_to_server,
        languages_server_to_client,
        first_kex_packet_follows,
    })
}

pub fn encode_binary_packet(payload: &[u8]) -> Vec<u8> {
    let block_size = 8usize;
    let mut padding_len = block_size - ((payload.len() + 5) % block_size);
    if padding_len < 4 {
        padding_len += block_size;
    }
    let packet_len = payload.len() + padding_len + 1;
    let mut packet = Vec::with_capacity(packet_len + 4);
    packet.extend_from_slice(&(packet_len as u32).to_be_bytes());
    packet.push(padding_len as u8);
    packet.extend_from_slice(payload);
    packet.resize(packet.len() + padding_len, 0);
    packet
}

pub fn parse_binary_packet(packet: &[u8]) -> Result<Vec<u8>, SshError> {
    if packet.len() < 6 {
        return Err(SshError::BadPacket);
    }
    let packet_len = u32::from_be_bytes([packet[0], packet[1], packet[2], packet[3]]) as usize;
    if packet_len + 4 != packet.len() {
        return Err(SshError::BadPacket);
    }
    let padding_len = packet[4] as usize;
    if padding_len < 4 || padding_len + 1 > packet_len {
        return Err(SshError::BadPacket);
    }
    let payload_len = packet_len - padding_len - 1;
    Ok(packet[5..5 + payload_len].to_vec())
}

pub fn default_client_kexinit(cookie: [u8; 16]) -> SshKexInit {
    SshKexInit {
        cookie,
        kex_algorithms: names(&["curve25519-sha256", "curve25519-sha256@libssh.org"]),
        server_host_key_algorithms: names(&["ssh-ed25519"]),
        encryption_algorithms_client_to_server: names(&["aes256-ctr"]),
        encryption_algorithms_server_to_client: names(&["aes256-ctr"]),
        mac_algorithms_client_to_server: names(&["hmac-sha2-256"]),
        mac_algorithms_server_to_client: names(&["hmac-sha2-256"]),
        compression_algorithms_client_to_server: names(&["none"]),
        compression_algorithms_server_to_client: names(&["none"]),
        languages_client_to_server: Vec::new(),
        languages_server_to_client: Vec::new(),
        first_kex_packet_follows: false,
    }
}

pub fn choose_algorithms(
    client: &SshKexInit,
    server: &SshKexInit,
) -> Result<SshAlgorithmSelection, SshError> {
    Ok(SshAlgorithmSelection {
        kex: choose_name(&client.kex_algorithms, &server.kex_algorithms)?,
        host_key: choose_name(
            &client.server_host_key_algorithms,
            &server.server_host_key_algorithms,
        )?,
        encryption_client_to_server: choose_name(
            &client.encryption_algorithms_client_to_server,
            &server.encryption_algorithms_client_to_server,
        )?,
        encryption_server_to_client: choose_name(
            &client.encryption_algorithms_server_to_client,
            &server.encryption_algorithms_server_to_client,
        )?,
        mac_client_to_server: choose_name(
            &client.mac_algorithms_client_to_server,
            &server.mac_algorithms_client_to_server,
        )?,
        mac_server_to_client: choose_name(
            &client.mac_algorithms_server_to_client,
            &server.mac_algorithms_server_to_client,
        )?,
        compression_client_to_server: choose_name(
            &client.compression_algorithms_client_to_server,
            &server.compression_algorithms_client_to_server,
        )?,
        compression_server_to_client: choose_name(
            &client.compression_algorithms_server_to_client,
            &server.compression_algorithms_server_to_client,
        )?,
    })
}

fn choose_name(client: &[String], server: &[String]) -> Result<String, SshError> {
    for name in client {
        if server.iter().any(|candidate| candidate == name) {
            return Ok(name.clone());
        }
    }
    Err(SshError::UnsupportedAlgorithm)
}

fn names(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub fn write_name_list(out: &mut Vec<u8>, names: &[String]) {
    let mut bytes = Vec::new();
    for (idx, name) in names.iter().enumerate() {
        if idx > 0 {
            bytes.push(b',');
        }
        bytes.extend_from_slice(name.as_bytes());
    }
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(&bytes);
}

pub fn write_string(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
}

pub fn write_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
}

pub fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub fn read_u32(input: &[u8], cursor: &mut usize) -> Result<u32, SshError> {
    if *cursor + 4 > input.len() {
        return Err(SshError::BadPacket);
    }
    let value = u32::from_be_bytes([
        input[*cursor],
        input[*cursor + 1],
        input[*cursor + 2],
        input[*cursor + 3],
    ]);
    *cursor += 4;
    Ok(value)
}

pub fn read_bool(input: &[u8], cursor: &mut usize) -> Result<bool, SshError> {
    if *cursor >= input.len() {
        return Err(SshError::BadPacket);
    }
    let value = input[*cursor] != 0;
    *cursor += 1;
    Ok(value)
}

pub fn read_string<'a>(input: &'a [u8], cursor: &mut usize) -> Result<&'a [u8], SshError> {
    let len = read_u32(input, cursor)? as usize;
    if *cursor + len > input.len() {
        return Err(SshError::BadPacket);
    }
    let value = &input[*cursor..*cursor + len];
    *cursor += len;
    Ok(value)
}

pub fn mpint_bytes(bytes: &[u8]) -> Vec<u8> {
    let first_nonzero = bytes
        .iter()
        .position(|value| *value != 0)
        .unwrap_or(bytes.len());
    let value = &bytes[first_nonzero..];
    if value.is_empty() {
        return vec![0, 0, 0, 0];
    }
    let mut out = Vec::new();
    let needs_zero = value[0] & 0x80 != 0;
    out.extend_from_slice(&((value.len() + usize::from(needs_zero)) as u32).to_be_bytes());
    if needs_zero {
        out.push(0);
    }
    out.extend_from_slice(value);
    out
}

pub fn read_name_list(input: &[u8], cursor: &mut usize) -> Result<Vec<String>, SshError> {
    if *cursor + 4 > input.len() {
        return Err(SshError::BadPacket);
    }
    let len = u32::from_be_bytes([
        input[*cursor],
        input[*cursor + 1],
        input[*cursor + 2],
        input[*cursor + 3],
    ]) as usize;
    *cursor += 4;
    if *cursor + len > input.len() {
        return Err(SshError::BadPacket);
    }
    let bytes = &input[*cursor..*cursor + len];
    *cursor += len;
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let text = core::str::from_utf8(bytes).map_err(|_| SshError::BadPacket)?;
    Ok(text.split(',').map(str::to_string).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_target() {
        assert_eq!(
            SshTarget::parse("root@example.com:2200").unwrap(),
            SshTarget {
                user: "root".to_string(),
                host: "example.com".to_string(),
                port: 2200
            }
        );
    }

    #[test]
    fn kexinit_roundtrips_through_packet() {
        let kex = default_client_kexinit([7; 16]);
        let payload = encode_kexinit(&kex);
        let packet = encode_binary_packet(&payload);
        let parsed_payload = parse_binary_packet(&packet).unwrap();
        assert_eq!(parsed_payload, payload);
        let parsed = parse_kexinit(&parsed_payload).unwrap();
        assert_eq!(parsed, kex);
    }
}
