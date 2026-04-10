//! HTTP/2 settings (RFC 7540 Section 6.5)

use super::{Http2Error, Result};

/// HTTP/2 setting identifiers
pub mod setting_ids {
    /// Header table size
    pub const HEADER_TABLE_SIZE: u16 = 0x1;
    /// Enable push
    pub const ENABLE_PUSH: u16 = 0x2;
    /// Max concurrent streams
    pub const MAX_CONCURRENT_STREAMS: u16 = 0x3;
    /// Initial window size
    pub const INITIAL_WINDOW_SIZE: u16 = 0x4;
    /// Max frame size
    pub const MAX_FRAME_SIZE: u16 = 0x5;
    /// Max header list size
    pub const MAX_HEADER_LIST_SIZE: u16 = 0x6;
}

/// HTTP/2 settings
#[derive(Debug, Clone)]
pub struct Settings {
    /// Header table size (default: 4096)
    pub header_table_size: u32,
    /// Enable push (default: 1)
    pub enable_push: u32,
    /// Max concurrent streams (default: unlimited)
    pub max_concurrent_streams: Option<u32>,
    /// Initial window size (default: 65535)
    pub initial_window_size: u32,
    /// Max frame size (default: 16384)
    pub max_frame_size: u32,
    /// Max header list size (default: unlimited)
    pub max_header_list_size: Option<u32>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            header_table_size: 4096,
            enable_push: 1,
            max_concurrent_streams: None,
            initial_window_size: 65535,
            max_frame_size: 16384,
            max_header_list_size: None,
        }
    }
}

impl Settings {
    /// Create default settings
    pub fn new() -> Self {
        Settings::default()
    }

    /// Convert to SETTINGS frame payload entries
    pub fn to_entries(&self) -> Vec<(u16, u32)> {
        let mut entries = Vec::new();

        entries.push((setting_ids::HEADER_TABLE_SIZE, self.header_table_size));
        entries.push((setting_ids::ENABLE_PUSH, self.enable_push));

        if let Some(max) = self.max_concurrent_streams {
            entries.push((setting_ids::MAX_CONCURRENT_STREAMS, max));
        }

        entries.push((setting_ids::INITIAL_WINDOW_SIZE, self.initial_window_size));
        entries.push((setting_ids::MAX_FRAME_SIZE, self.max_frame_size));

        if let Some(max) = self.max_header_list_size {
            entries.push((setting_ids::MAX_HEADER_LIST_SIZE, max));
        }

        entries
    }

    /// Parse from SETTINGS frame entries
    pub fn from_entries(entries: &[(u16, u32)]) -> Result<Self> {
        let mut settings = Settings::default();

        for &(id, value) in entries {
            match id {
                setting_ids::HEADER_TABLE_SIZE => {
                    settings.header_table_size = value;
                }
                setting_ids::ENABLE_PUSH => {
                    if value > 1 {
                        return Err(Http2Error::ProtocolViolation(format!(
                            "Invalid ENABLE_PUSH value: {}",
                            value
                        )));
                    }
                    settings.enable_push = value;
                }
                setting_ids::MAX_CONCURRENT_STREAMS => {
                    settings.max_concurrent_streams = Some(value);
                }
                setting_ids::INITIAL_WINDOW_SIZE => {
                    if value > 2147483647 {
                        return Err(Http2Error::FlowControl(format!(
                            "INITIAL_WINDOW_SIZE too large: {}",
                            value
                        )));
                    }
                    settings.initial_window_size = value;
                }
                setting_ids::MAX_FRAME_SIZE => {
                    if value < 16384 || value > 16777215 {
                        return Err(Http2Error::ProtocolViolation(format!(
                            "Invalid MAX_FRAME_SIZE: {}",
                            value
                        )));
                    }
                    settings.max_frame_size = value;
                }
                setting_ids::MAX_HEADER_LIST_SIZE => {
                    settings.max_header_list_size = Some(value);
                }
                _ => {
                    // Unknown setting - ignore per RFC
                }
            }
        }

        Ok(settings)
    }

    /// Validate settings
    pub fn validate(&self) -> Result<()> {
        if self.max_frame_size < 16384 {
            return Err(Http2Error::ProtocolViolation(
                "MAX_FRAME_SIZE must be >= 16384".to_string(),
            ));
        }

        if self.max_frame_size > 16777215 {
            return Err(Http2Error::ProtocolViolation(
                "MAX_FRAME_SIZE must be <= 16777215".to_string(),
            ));
        }

        if self.initial_window_size > 2147483647 {
            return Err(Http2Error::FlowControl(
                "INITIAL_WINDOW_SIZE must be <= 2^31-1".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::new();
        assert_eq!(settings.header_table_size, 4096);
        assert_eq!(settings.enable_push, 1);
        assert!(settings.max_concurrent_streams.is_none());
        assert_eq!(settings.initial_window_size, 65535);
        assert_eq!(settings.max_frame_size, 16384);
        assert!(settings.max_header_list_size.is_none());
    }

    #[test]
    fn test_settings_to_entries() {
        let settings = Settings::new();
        let entries = settings.to_entries();

        // Should have 4 mandatory settings
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].0, setting_ids::HEADER_TABLE_SIZE);
        assert_eq!(entries[0].1, 4096);
    }

    #[test]
    fn test_settings_from_entries() {
        let entries = vec![
            (setting_ids::HEADER_TABLE_SIZE, 8192),
            (setting_ids::MAX_FRAME_SIZE, 32768),
        ];

        let settings = Settings::from_entries(&entries).unwrap();
        assert_eq!(settings.header_table_size, 8192);
        assert_eq!(settings.max_frame_size, 32768);
    }

    #[test]
    fn test_settings_invalid_enable_push() {
        let entries = vec![(setting_ids::ENABLE_PUSH, 2)];
        assert!(Settings::from_entries(&entries).is_err());
    }

    #[test]
    fn test_settings_invalid_max_frame_size() {
        let mut settings = Settings::new();
        settings.max_frame_size = 100; // Too small
        assert!(settings.validate().is_err());

        settings.max_frame_size = 20000000; // Too large
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_settings_unknown_id() {
        // Unknown settings should be ignored
        let entries = vec![(0xFF, 1234)];
        let settings = Settings::from_entries(&entries).unwrap();
        assert_eq!(settings.max_frame_size, 16384); // Default unchanged
    }
}
