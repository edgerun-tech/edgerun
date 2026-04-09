use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NfcPowerState {
    Unknown,
    Enabled,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NfcTechnology {
    NfcA,   // ISO 14443-3A, Mifare Ultralight, Mifare DESFire
    NfcB,   // ISO 14443-3B
    NfcF,   // FeliCa (JIS 6319-4)
    NfcV,   // ISO 15693
    IsoDep, // ISO 14443-4 (T=CL)
    Mifare, // Mifare Classic
    Unknown,
}

impl NfcTechnology {
    pub fn as_str(&self) -> &'static str {
        match self {
            NfcTechnology::NfcA => "NFC-A",
            NfcTechnology::NfcB => "NFC-B",
            NfcTechnology::NfcF => "NFC-F",
            NfcTechnology::NfcV => "NFC-V",
            NfcTechnology::IsoDep => "ISO-DEP",
            NfcTechnology::Mifare => "Mifare",
            NfcTechnology::Unknown => "Unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "iso14443" | "iso14443a" | "nfc-a" | "nfca" => NfcTechnology::NfcA,
            "iso14443b" | "nfc-b" | "nfcb" => NfcTechnology::NfcB,
            "felica" | "nfc-f" | "nfcf" | "jis6319-4" => NfcTechnology::NfcF,
            "iso15693" | "nfc-v" | "nfcv" => NfcTechnology::NfcV,
            "iso-dep" | "isodep" | "iso14443-4" => NfcTechnology::IsoDep,
            "mifare" | "mifare-classic" => NfcTechnology::Mifare,
            _ => NfcTechnology::Unknown,
        }
    }
}

// ---------------------------------------------------------------------------
// NDEF types
// ---------------------------------------------------------------------------

/// An NDEF record within an NDEF message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NdefRecord {
    pub tnf: NdefTnf,
    pub type_name: Vec<u8>,
    pub id: Vec<u8>,
    pub payload: Vec<u8>,
}

impl NdefRecord {
    /// Creates a well-known text record (TNF=1, type="T").
    /// Language code defaults to "en".
    pub fn text_record(text: &str, language_code: &str) -> Self {
        let mut payload = Vec::new();
        payload.push(language_code.len() as u8);
        payload.extend_from_slice(language_code.as_bytes());
        payload.extend_from_slice(text.as_bytes());
        NdefRecord {
            tnf: NdefTnf::NfcWellKnown,
            type_name: b"T".to_vec(),
            id: Vec::new(),
            payload,
        }
    }

    /// Creates a URI record (TNF=1, type="U").
    pub fn uri_record(uri: &str) -> Self {
        NdefRecord {
            tnf: NdefTnf::NfcWellKnown,
            type_name: b"U".to_vec(),
            id: Vec::new(),
            payload: uri.as_bytes().to_vec(),
        }
    }

    /// Creates a MIME media record with arbitrary payload.
    pub fn mime_record(mime_type: &str, data: &[u8]) -> Self {
        NdefRecord {
            tnf: NdefTnf::Media,
            type_name: mime_type.as_bytes().to_vec(),
            id: Vec::new(),
            payload: data.to_vec(),
        }
    }

    /// If this record is a well-known text record, decode it.
    pub fn as_text(&self) -> Option<String> {
        if self.tnf != NdefTnf::NfcWellKnown || self.type_name != b"T" {
            return None;
        }
        if self.payload.is_empty() {
            return None;
        }
        let lang_len = self.payload[0] as usize & 0x3F;
        if lang_len + 1 >= self.payload.len() {
            return None;
        }
        let text = &self.payload[lang_len + 1..];
        String::from_utf8(text.to_vec()).ok()
    }

    /// If this record is a well-known URI record, decode it.
    pub fn as_uri(&self) -> Option<String> {
        if self.tnf != NdefTnf::NfcWellKnown || self.type_name != b"U" {
            return None;
        }
        String::from_utf8(self.payload.clone()).ok()
    }
}

/// NDEF Type Name Format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NdefTnf {
    Empty = 0x00,
    NfcWellKnown = 0x01,
    Media = 0x02,
    AbsoluteUri = 0x03,
    NfcExternal = 0x04,
    Unknown = 0x05,
    Unchanged = 0x06,
    Reserved = 0x07,
}

impl NdefTnf {
    pub fn from_u8(v: u8) -> Self {
        match v & 0x07 {
            0x00 => NdefTnf::Empty,
            0x01 => NdefTnf::NfcWellKnown,
            0x02 => NdefTnf::Media,
            0x03 => NdefTnf::AbsoluteUri,
            0x04 => NdefTnf::NfcExternal,
            0x05 => NdefTnf::Unknown,
            0x06 => NdefTnf::Unchanged,
            _ => NdefTnf::Reserved,
        }
    }
}

/// An NDEF message containing one or more records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NdefMessage {
    pub records: Vec<NdefRecord>,
}

impl NdefMessage {
    pub fn new(records: Vec<NdefRecord>) -> Self {
        Self { records }
    }

    pub fn single_text(text: &str) -> Self {
        Self {
            records: vec![NdefRecord::text_record(text, "en")],
        }
    }

    /// Serialize this NDEF message to raw bytes per NFC Forum NDEF spec.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let last = self.records.len().saturating_sub(1);
        for (i, record) in self.records.iter().enumerate() {
            let is_first = i == 0;
            let is_last = i == last;
            let tnf = record.tnf as u8 & 0x07;

            // Short Record: payload length fits in 1 byte (<= 255)
            let is_short = record.payload.len() <= 255;
            let has_id = !record.id.is_empty() && is_first;

            let mut flags = tnf;
            if is_first { flags |= 0x80; } // MB
            if is_last  { flags |= 0x40; } // ME
            if is_short { flags |= 0x10; } // SR
            if has_id   { flags |= 0x08; } // IL

            out.push(flags);
            out.push(record.type_name.len() as u8);

            if is_short {
                out.push(record.payload.len() as u8);
            } else {
                let len = record.payload.len() as u32;
                out.extend_from_slice(&len.to_be_bytes());
            }

            if has_id {
                out.push(record.id.len() as u8);
            }

            out.extend_from_slice(&record.type_name);
            out.extend_from_slice(&record.id);
            out.extend_from_slice(&record.payload);
        }
        out
    }

    /// Parse an NDEF message from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let mut records = Vec::new();
        let mut pos = 0;
        let mut message_end = false;

        while pos < data.len() && !message_end {
            let flags = data[pos];
            pos += 1;

            let tnf = NdefTnf::from_u8(flags);
            let is_first = (flags & 0x80) != 0;
            message_end = (flags & 0x40) != 0;
            let is_short = (flags & 0x10) != 0;
            let has_id = (flags & 0x08) != 0;

            // Type length — every record has one
            if pos >= data.len() {
                return None;
            }
            let type_len = data[pos] as usize;
            pos += 1;

            // Payload length
            let payload_len = if is_short {
                if pos >= data.len() {
                    return None;
                }
                let len = data[pos] as usize;
                pos += 1;
                len
            } else {
                if pos + 4 > data.len() {
                    return None;
                }
                let len = u32::from_be_bytes([
                    data[pos],
                    data[pos + 1],
                    data[pos + 2],
                    data[pos + 3],
                ]) as usize;
                pos += 4;
                len
            };

            // ID length (only present if IL flag is set)
            let id_len = if has_id {
                if pos >= data.len() {
                    return None;
                }
                let len = data[pos] as usize;
                pos += 1;
                len
            } else {
                0
            };

            if pos + type_len > data.len() {
                return None;
            }
            let type_name = data[pos..pos + type_len].to_vec();
            pos += type_len;

            if pos + id_len > data.len() {
                return None;
            }
            let id = data[pos..pos + id_len].to_vec();
            pos += id_len;

            if pos + payload_len > data.len() {
                return None;
            }
            let payload = data[pos..pos + payload_len].to_vec();
            pos += payload_len;

            records.push(NdefRecord {
                tnf,
                type_name,
                id,
                payload,
            });
        }

        if records.is_empty() {
            None
        } else {
            Some(NdefMessage { records })
        }
    }
}

// ---------------------------------------------------------------------------
// Device and target types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NfcDeviceInfo {
    pub provider: String,
    pub device_name: String,
    pub power_state: NfcPowerState,
    pub supported_technologies: Vec<NfcTechnology>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NfcTarget {
    /// Unique identifier for this target session.
    pub target_id: String,
    /// NFC technology detected (NFC-A, NFC-B, FeliCa, etc.).
    pub technology: NfcTechnology,
    /// ATQA/SENS_RES or equivalent initial response bytes.
    pub atqa: Vec<u8>,
    /// UID / NFCID1 / NFCID2 of the tag.
    pub uid: Vec<u8>,
    /// Historical bytes / ATS if available.
    pub historical_bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NfcTargetObservation {
    pub target_id: String,
    pub technology: NfcTechnology,
    pub uid: Vec<u8>,
    pub ndef_message: Option<NdefMessage>,
    pub is_readable: bool,
    pub is_writable: bool,
    pub observed_at_unix_ms: i64,
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

pub trait NfcDevice: CapabilityProvider {
    /// Returns information about this NFC adapter.
    fn device_info(&self) -> Result<NfcDeviceInfo, edgerun_capabilities::CapabilityError>;

    /// Returns the current power state of the adapter.
    fn power_state(&self) -> Result<NfcPowerState, edgerun_capabilities::CapabilityError>;

    /// Powers the NFC field on or off.
    fn set_power_state(
        &self,
        state: NfcPowerState,
    ) -> Result<NfcPowerState, edgerun_capabilities::CapabilityError>;
}

pub trait NfcScanner: CapabilityProvider {
    /// Scans for nearby NFC targets. Returns immediately with any targets found.
    fn scan_targets(&self) -> Result<Vec<NfcTarget>, edgerun_capabilities::CapabilityError>;
}

pub trait NfcReader: CapabilityProvider {
    /// Reads an NDEF message from a discovered target.
    fn read_ndef(
        &self,
        target_id: &str,
    ) -> Result<Option<NdefMessage>, edgerun_capabilities::CapabilityError>;

    /// Writes an NDEF message to a discovered target.
    fn write_ndef(
        &self,
        target_id: &str,
        message: &NdefMessage,
    ) -> Result<(), edgerun_capabilities::CapabilityError>;

    /// Sends raw APDU / transceive command to a target (ISO-DEP, NFC-A, etc.).
    fn transceive(
        &self,
        target_id: &str,
        command: &[u8],
    ) -> Result<Vec<u8>, edgerun_capabilities::CapabilityError>;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn default_nfc_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Communication,
        &[CapabilityModality::Radio],
        &[CapabilityEventKind::Radio, CapabilityEventKind::State],
        &[
            CapabilityOperation::Query,
            CapabilityOperation::Observe,
            CapabilityOperation::Control,
            CapabilityOperation::Invoke,
        ],
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- NfcPowerState ---

    #[test]
    fn nfc_power_state_variants_are_copy() {
        let state = NfcPowerState::Enabled;
        let _copied = state;
        assert_eq!(state, NfcPowerState::Enabled);
    }

    #[test]
    fn nfc_power_state_equality() {
        assert_eq!(NfcPowerState::Unknown, NfcPowerState::Unknown);
        assert_ne!(NfcPowerState::Enabled, NfcPowerState::Disabled);
    }

    // --- NfcTechnology ---

    #[test]
    fn nfc_technology_from_str() {
        assert_eq!(NfcTechnology::from_str("NFC-A"), NfcTechnology::NfcA);
        assert_eq!(NfcTechnology::from_str("iso14443a"), NfcTechnology::NfcA);
        assert_eq!(NfcTechnology::from_str("FeliCa"), NfcTechnology::NfcF);
        assert_eq!(NfcTechnology::from_str("iso15693"), NfcTechnology::NfcV);
        assert_eq!(NfcTechnology::from_str("ISO-DEP"), NfcTechnology::IsoDep);
        assert_eq!(NfcTechnology::from_str("bogus"), NfcTechnology::Unknown);
    }

    #[test]
    fn nfc_technology_as_str() {
        assert_eq!(NfcTechnology::NfcA.as_str(), "NFC-A");
        assert_eq!(NfcTechnology::NfcF.as_str(), "NFC-F");
    }

    // --- NDEF ---

    #[test]
    fn ndef_text_roundtrip() {
        let msg = NdefMessage::single_text("Hello World");
        let bytes = msg.to_bytes();
        let parsed = NdefMessage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.records[0].as_text().as_deref(), Some("Hello World"));
    }

    #[test]
    fn ndef_uri_roundtrip() {
        let msg = NdefMessage {
            records: vec![NdefRecord::uri_record("https://example.com")],
        };
        let bytes = msg.to_bytes();
        let parsed = NdefMessage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.records[0].as_uri().as_deref(), Some("https://example.com"));
    }

    #[test]
    fn ndef_mime_roundtrip() {
        let msg = NdefMessage {
            records: vec![NdefRecord::mime_record("application/json", b"{\"key\":\"value\"}")],
        };
        let bytes = msg.to_bytes();
        let parsed = NdefMessage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.records[0].tnf, NdefTnf::Media);
        assert_eq!(parsed.records[0].type_name, b"application/json");
        assert_eq!(parsed.records[0].payload, b"{\"key\":\"value\"}");
    }

    #[test]
    fn ndef_multi_record() {
        let msg = NdefMessage {
            records: vec![
                NdefRecord::text_record("Hello", "en"),
                NdefRecord::uri_record("https://example.com"),
            ],
        };
        let bytes = msg.to_bytes();
        let parsed = NdefMessage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.records.len(), 2);
    }

    #[test]
    fn ndef_empty_returns_none() {
        assert!(NdefMessage::from_bytes(&[]).is_none());
    }

    #[test]
    fn ndef_corrupted_returns_none() {
        assert!(NdefMessage::from_bytes(&[0xFF, 0xFF]).is_none());
    }

    // --- NdefRecord ---

    #[test]
    fn ndef_record_text_decode() {
        let record = NdefRecord::text_record("test", "en");
        assert_eq!(record.as_text().as_deref(), Some("test"));
    }

    #[test]
    fn ndef_record_uri_decode() {
        let record = NdefRecord::uri_record("http://foo.bar");
        assert_eq!(record.as_uri().as_deref(), Some("http://foo.bar"));
    }

    #[test]
    fn ndef_record_non_text_returns_none() {
        let record = NdefRecord::uri_record("http://x");
        assert!(record.as_text().is_none());
    }

    // --- Types ---

    #[test]
    fn nfc_device_info_construction() {
        let info = NfcDeviceInfo {
            provider: "linux-nfc".into(),
            device_name: "nfc0".into(),
            power_state: NfcPowerState::Enabled,
            supported_technologies: vec![NfcTechnology::NfcA, NfcTechnology::IsoDep],
        };
        assert_eq!(info.device_name, "nfc0");
        assert_eq!(info.supported_technologies.len(), 2);
    }

    #[test]
    fn nfc_target_construction() {
        let target = NfcTarget {
            target_id: "tag-1".into(),
            technology: NfcTechnology::NfcA,
            atqa: vec![0x04, 0x00],
            uid: vec![0x04, 0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56],
            historical_bytes: Vec::new(),
        };
        assert_eq!(target.target_id, "tag-1");
        assert_eq!(target.uid.len(), 7);
    }

    #[test]
    fn nfc_target_observation_construction() {
        let obs = NfcTargetObservation {
            target_id: "tag-1".into(),
            technology: NfcTechnology::NfcA,
            uid: vec![0x04, 0xAB],
            ndef_message: Some(NdefMessage::single_text("hello")),
            is_readable: true,
            is_writable: false,
            observed_at_unix_ms: 1_700_000_000_000,
        };
        assert!(obs.ndef_message.is_some());
        assert!(obs.is_readable);
        assert!(!obs.is_writable);
    }

    // --- Descriptor ---

    #[test]
    fn nfc_descriptor_has_invoke_operation() {
        let descriptor = default_nfc_descriptor("nfc", "nfc0");
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Invoke as i32)));
    }
}
