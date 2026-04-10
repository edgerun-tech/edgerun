//! HTTP/3 settings (RFC 9114 Section 7.2.4.1)

/// Setting identifiers
pub mod setting_ids {
    /// QPACK maximum table capacity
    pub const MAX_TABLE_CAPACITY: u64 = 0x06;
    /// QPACK maximum blocked streams
    pub const MAX_BLOCKED_STREAMS: u64 = 0x07;
    /// QPACK encoder maximum memory usage
    pub const ENCODER_MAX_MEMORY_USAGE: u64 = 0x08; // Not standardized, using grease
    /// Max header list size (same as HTTP/2)
    pub const MAX_HEADER_LIST_SIZE: u64 = 0x08; // Actually 0x08 is different in HTTP/3
}

/// HTTP/3 settings
#[derive(Debug, Clone)]
pub struct Http3Settings {
    /// QPACK maximum table capacity
    pub max_table_capacity: u64,
    /// QPACK maximum blocked streams
    pub max_blocked_streams: u64,
    /// Max header list size
    pub max_header_list_size: Option<u64>,
}

impl Default for Http3Settings {
    fn default() -> Self {
        Http3Settings {
            max_table_capacity: 4096,
            max_blocked_streams: 100,
            max_header_list_size: None,
        }
    }
}

impl Http3Settings {
    /// Create default settings
    pub fn new() -> Self {
        Http3Settings::default()
    }

    /// Convert to entries
    pub fn to_entries(&self) -> Vec<(u64, u64)> {
        let mut entries = vec![
            (setting_ids::MAX_TABLE_CAPACITY, self.max_table_capacity),
            (setting_ids::MAX_BLOCKED_STREAMS, self.max_blocked_streams),
        ];

        if let Some(max) = self.max_header_list_size {
            entries.push((0x09, max)); // Max header list size
        }

        entries
    }

    /// Parse from entries
    pub fn from_entries(entries: &[(u64, u64)]) -> Result<Self, String> {
        let mut settings = Http3Settings::default();

        for &(id, value) in entries {
            match id {
                setting_ids::MAX_TABLE_CAPACITY => {
                    settings.max_table_capacity = value;
                }
                setting_ids::MAX_BLOCKED_STREAMS => {
                    settings.max_blocked_streams = value;
                }
                0x09 => {
                    settings.max_header_list_size = Some(value);
                }
                _ => {
                    // Unknown setting - ignore per RFC
                }
            }
        }

        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Http3Settings::new();
        assert_eq!(settings.max_table_capacity, 4096);
        assert_eq!(settings.max_blocked_streams, 100);
        assert!(settings.max_header_list_size.is_none());
    }

    #[test]
    fn test_settings_to_entries() {
        let settings = Http3Settings::new();
        let entries = settings.to_entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, setting_ids::MAX_TABLE_CAPACITY);
    }

    #[test]
    fn test_settings_from_entries() {
        let entries = vec![
            (setting_ids::MAX_TABLE_CAPACITY, 8192),
            (setting_ids::MAX_BLOCKED_STREAMS, 50),
        ];
        let settings = Http3Settings::from_entries(&entries).unwrap();
        assert_eq!(settings.max_table_capacity, 8192);
        assert_eq!(settings.max_blocked_streams, 50);
    }
}
