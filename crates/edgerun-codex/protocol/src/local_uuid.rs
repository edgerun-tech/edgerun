use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use ts_rs::TS;

static COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Uuid([u8; 16]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error;

impl Uuid {
    pub fn new_v4() -> Self {
        let mut bytes = entropy_bytes();
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self(bytes)
    }

    pub fn now_v7() -> Self {
        let mut bytes = entropy_bytes();
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or_default();
        bytes[0] = (millis >> 40) as u8;
        bytes[1] = (millis >> 32) as u8;
        bytes[2] = (millis >> 24) as u8;
        bytes[3] = (millis >> 16) as u8;
        bytes[4] = (millis >> 8) as u8;
        bytes[5] = millis as u8;
        bytes[6] = (bytes[6] & 0x0f) | 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self(bytes)
    }

    pub const fn nil() -> Self {
        Self([0; 16])
    }

    pub fn parse_str(input: &str) -> Result<Self, Error> {
        let bytes = input.as_bytes();
        if bytes.len() != 36 {
            return Err(Error);
        }
        for idx in [8, 13, 18, 23] {
            if bytes[idx] != b'-' {
                return Err(Error);
            }
        }
        let mut out = [0u8; 16];
        let mut input_idx = 0usize;
        for byte in &mut out {
            while input_idx < bytes.len() && bytes[input_idx] == b'-' {
                input_idx += 1;
            }
            let high = hex_value(bytes.get(input_idx).copied().ok_or(Error)?)?;
            let low = hex_value(bytes.get(input_idx + 1).copied().ok_or(Error)?)?;
            *byte = (high << 4) | low;
            input_idx += 2;
        }
        Ok(Self(out))
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, byte) in self.0.iter().enumerate() {
            if matches!(idx, 4 | 6 | 8 | 10) {
                f.write_str("-")?;
            }
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid UUID")
    }
}

impl std::error::Error for Error {}

impl TS for Uuid {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn decl() -> String {
        "type Uuid = string;".to_string()
    }

    fn name() -> String {
        "string".to_string()
    }

    fn inline() -> String {
        "string".to_string()
    }
}

fn entropy_bytes() -> [u8; 16] {
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let pid = std::process::id() as u128;
    let mixed = nanos ^ ((counter as u128) << 64) ^ pid.rotate_left(17);
    mixed.to_be_bytes()
}

fn hex_value(byte: u8) -> Result<u8, Error> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(Error),
    }
}
