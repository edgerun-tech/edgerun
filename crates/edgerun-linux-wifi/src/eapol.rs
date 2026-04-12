#![allow(dead_code)]
#![allow(unused_must_use)]

use edgerun_capabilities::CapabilityError;
use edgerun_crypto::sha2::Digest;
use std::time::{SystemTime, UNIX_EPOCH};
// WPA/WPA2 PSK derivation and EAPOL Key handling
// ============================================================================

/// Parse a MAC address string like "AA:BB:CC:DD:EE:FF" into bytes.
pub fn parse_mac_string(s: &str) -> Option<[u8; 6]> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return None;
    }
    let mut mac = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        mac[i] = u8::from_str_radix(part, 16).ok()?;
    }
    Some(mac)
}

/// Derive the Pairwise Master Key (PMK) from a WPA passphrase using PBKDF2-SHA1.
///
/// This implements the algorithm from IEEE 802.11i / WPA spec:
/// PMK = PBKDF2-SHA1(passphrase, SSID, SSID_len, 4096, 256)
///
/// The `passphrase` should be 8-63 ASCII characters.
/// The `ssid` is the network SSID as bytes.
/// Returns a 32-byte PMK.
pub fn derive_wpa_pmk(passphrase: &str, ssid: &[u8]) -> [u8; 32] {
    use edgerun_crypto::pbkdf2::pbkdf2;
    use edgerun_crypto::sha1::Sha1;
    use edgerun_crypto::hmac::Hmac;

    let mut pmk = [0u8; 32];
    // WPA uses 4096 iterations per spec
    pbkdf2::<Hmac<Sha1>>(passphrase.as_bytes(), ssid, 4096, &mut pmk);
    pmk
}

/// EAPOL-Key frame constants
const EAPOL_KEY_TYPE_RSN: u8 = 2;
const EAPOL_KEY_INFO_TYPE_MASK: u16 = 0x0007;
const EAPOL_KEY_INFO_KEY_TYPE: u16 = 1 << 3; // Pairwise (1) or Group (0)
const EAPOL_KEY_INFO_INSTALL: u16 = 1 << 6;
const EAPOL_KEY_INFO_ACK: u16 = 1 << 7;
const EAPOL_KEY_INFO_MIC: u16 = 1 << 8;
const EAPOL_KEY_INFO_SECURE: u16 = 1 << 9;
const EAPOL_KEY_INFO_ENCRYPTED: u16 = 1 << 10;

/// WPA 4-way handshake message types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EapolKeyMessageType {
    Message1, // AP -> STA: ANonce
    Message2, // STA -> AP: SNonce + MIC
    Message3, // AP -> STA: GTK + MIC + Install
    Message4, // STA -> AP: Confirmation
}

/// Parsed EAPOL-Key frame.
#[derive(Clone, Debug)]
pub struct ParsedEapolKey {
    pub key_info: u16,
    pub key_length: u16,
    pub replay_counter: u64,
    pub key_nonce: [u8; 32],
    pub key_iv: [u8; 16],
    pub key_rsc: u64,
    pub key_id: u64,
    pub key_mic: [u8; 16],
    pub key_data_length: u16,
    pub key_data: Vec<u8>,
}

impl ParsedEapolKey {
    /// Determine which message of the 4-way handshake this is.
    pub fn message_type(&self) -> EapolKeyMessageType {
        let has_ack = (self.key_info & EAPOL_KEY_INFO_ACK) != 0;
        let has_mic = (self.key_info & EAPOL_KEY_INFO_MIC) != 0;
        let has_secure = (self.key_info & EAPOL_KEY_INFO_SECURE) != 0;

        match (has_ack, has_mic, has_secure) {
            (true, false, false) => EapolKeyMessageType::Message1,
            (false, true, false) => EapolKeyMessageType::Message2,
            (true, true, true) => EapolKeyMessageType::Message3,
            (false, true, true) => EapolKeyMessageType::Message4,
            _ => EapolKeyMessageType::Message1, // Default
        }
    }

    /// Check if this is a pairwise key (vs group key).
    pub fn is_pairwise(&self) -> bool {
        (self.key_info & EAPOL_KEY_INFO_KEY_TYPE) != 0
    }
}

/// Parse an EAPOL-Key frame from raw bytes.
///
/// The EAPOL-Key frame format (per IEEE 802.1X-2010):
/// - EAPOL header: protocol_version (1) + packet_type (1) + packet_body_length (2)
/// - Key descriptor: descriptor_type (1) + key_info (2) + key_length (2)
///   + replay_counter (8) + key_nonce (32) + key_iv (16) + key_rsc (8)
///   + key_id (8) + key_mic (16) + key_data_length (2) + key_data (variable)
pub fn parse_eapol_key_frame(data: &[u8]) -> Result<ParsedEapolKey, &'static str> {
    // Need at least EAPOL header (4) + Key descriptor header (77)
    if data.len() < 4 + 77 {
        return Err("EAPOL-Key frame too short");
    }

    // Skip EAPOL header
    let key_data = &data[4..];

    // Parse descriptor type (should be 2 for RSN)
    let descriptor_type = key_data[0];
    if descriptor_type != EAPOL_KEY_TYPE_RSN {
        return Err("not an RSN EAPOL-Key frame");
    }

    // Parse fields
    let key_info = u16::from_be_bytes([key_data[1], key_data[2]]);
    let key_length = u16::from_be_bytes([key_data[3], key_data[4]]);
    let replay_counter = u64::from_be_bytes([
        key_data[5], key_data[6], key_data[7], key_data[8],
        key_data[9], key_data[10], key_data[11], key_data[12],
    ]);

    let mut key_nonce = [0u8; 32];
    key_nonce.copy_from_slice(&key_data[13..45]);

    let mut key_iv = [0u8; 16];
    key_iv.copy_from_slice(&key_data[45..61]);

    let key_rsc = u64::from_be_bytes([
        key_data[61], key_data[62], key_data[63], key_data[64],
        key_data[65], key_data[66], key_data[67], key_data[68],
    ]);

    let key_id = u64::from_be_bytes([
        key_data[69], key_data[70], key_data[71], key_data[72],
        key_data[73], key_data[74], key_data[75], key_data[76],
    ]);

    let mut key_mic = [0u8; 16];
    key_mic.copy_from_slice(&key_data[77..93]);

    let key_data_length = u16::from_be_bytes([key_data[93], key_data[94]]);

    let key_data_payload = if key_data.len() >= 95 + key_data_length as usize {
        key_data[95..95 + key_data_length as usize].to_vec()
    } else {
        Vec::new()
    };

    Ok(ParsedEapolKey {
        key_info,
        key_length,
        replay_counter,
        key_nonce,
        key_iv,
        key_rsc,
        key_id,
        key_mic,
        key_data_length,
        key_data: key_data_payload,
    })
}

/// Derive the Pairwise Transient Key (PTK) for the 4-way handshake.
///
/// PTK = PRF-X(PMK, "Pairwise key expansion", Min(AA, SA) || Max(AA, SA) ||
///                  Min(ANonce, SNonce) || Max(ANonce, SNonce))
///
/// Where:
/// - PMK: Pairwise Master Key (from passphrase)
/// - AA: Authenticator Address (AP's BSSID)
/// - SA: Supplicant Address (STA's MAC)
/// - ANonce: Authenticator's nonce (from Message 1)
/// - SNonce: Supplicant's nonce (generated by STA)
/// - X: Key length (64 bytes for CCMP, 80 bytes for TKIP)
pub fn derive_ptk(
    pmk: &[u8; 32],
    authenticator_addr: &[u8; 6],
    supplicant_addr: &[u8; 6],
    anonce: &[u8; 32],
    snonce: &[u8; 32],
    key_length: usize,
) -> Vec<u8> {
    use edgerun_crypto::hmac::{Hmac, Mac};
    use edgerun_crypto::sha1::Sha1;

    // Construct the input for PRF
    let mut input = Vec::with_capacity(102);
    // Min(AA, SA) || Max(AA, SA)
    if authenticator_addr < supplicant_addr {
        input.extend_from_slice(authenticator_addr);
        input.extend_from_slice(supplicant_addr);
    } else {
        input.extend_from_slice(supplicant_addr);
        input.extend_from_slice(authenticator_addr);
    }
    // Min(ANonce, SNonce) || Max(ANonce, SNonce)
    if anonce < snonce {
        input.extend_from_slice(anonce);
        input.extend_from_slice(snonce);
    } else {
        input.extend_from_slice(snonce);
        input.extend_from_slice(anonce);
    }

    // PRF-X using HMAC-SHA1
    let label = b"Pairwise key expansion";
    let mut ptk = Vec::with_capacity(key_length);
    let mut counter = 0u8;

    while ptk.len() < key_length {
        let mut hmac = Hmac::<Sha1>::new_from_slice(pmk).expect("HMAC can take key of any size");
        hmac.update(label);
        hmac.update(&[0]); // Zero byte separator
        hmac.update(&[counter]);
        hmac.update(&input);
        let result = hmac.finalize().into_bytes();
        ptk.extend_from_slice(&result);
        counter += 1;
    }

    ptk.truncate(key_length);
    ptk
}

/// Calculate the MIC for an EAPOL-Key frame.
///
/// The MIC is computed over the entire EAPOL frame with the MIC field zeroed.
pub fn calculate_eapol_mic(
    ptk: &[u8],
    eapol_frame: &[u8],
) -> [u8; 16] {
    use edgerun_crypto::hmac::{Hmac, Mac};
    use edgerun_crypto::sha1::Sha1;

    // MIC is computed using the first 16 bytes of PTK (MIC Key)
    let mic_key = &ptk[..16];

    // Create a copy of the frame with MIC field zeroed
    // MIC field starts at offset 4 (EAPOL header) + 77 (key descriptor before MIC) = 81
    let mut frame = eapol_frame.to_vec();
    let mic_offset = 4 + 77; // After EAPOL header + descriptor fields before MIC
    for byte in frame.iter_mut().skip(mic_offset).take(16) {
        *byte = 0;
    }

    // Compute HMAC-SHA1
    let mut hmac = Hmac::<Sha1>::new_from_slice(mic_key).expect("HMAC can take key of any size");
    hmac.update(&frame);
    let result = hmac.finalize().into_bytes();

    let mut mic = [0u8; 16];
    mic.copy_from_slice(&result[..16]);
    mic
}

/// Verify the MIC in a received EAPOL-Key frame.
pub fn verify_eapol_mic(
    ptk: &[u8],
    eapol_frame: &[u8],
    expected_mic: &[u8; 16],
) -> bool {
    let computed_mic = calculate_eapol_mic(ptk, eapol_frame);
    computed_mic == *expected_mic
}
