#![no_std]

extern crate alloc;

use alloc::string::String;

pub use edgerun_encoding::rfc3339::DateTimeUtc;
pub use edgerun_encoding::rfc3339::ParseRfc3339Error;

pub fn parse_rfc3339_utc(value: &str) -> Result<DateTimeUtc, ParseRfc3339Error> {
    edgerun_encoding::rfc3339::parse_rfc3339(value)
}

pub fn format_rfc3339_utc(value: &DateTimeUtc) -> String {
    edgerun_encoding::rfc3339::format_rfc3339_utc(value)
}

pub fn canonical_rfc3339_utc(value: &str) -> Option<String> {
    edgerun_encoding::rfc3339::canonical_time_string(value)
}

#[cfg(feature = "chrono")]
pub mod chrono {
    use super::DateTimeUtc;
    use super::ParseRfc3339Error;
    use alloc::string::String;
    use chrono::DateTime;
    use chrono::Utc;
    use core::time::Duration;

    pub type ChronoUtcDateTime = DateTime<Utc>;
    pub type ChronoDuration = chrono::Duration;
    pub use chrono::Datelike as ChronoDatelike;
    pub use chrono::Local as ChronoLocal;
    pub use chrono::TimeZone as ChronoTimeZone;
    pub use chrono::Utc as ChronoUtc;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ChronoTimeError {
        InvalidRfc3339(ParseRfc3339Error),
        OutOfRange,
    }

    impl core::fmt::Display for ChronoTimeError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::InvalidRfc3339(error) => error.fmt(f),
                Self::OutOfRange => f.write_str("timestamp is out of chrono range"),
            }
        }
    }

    impl core::error::Error for ChronoTimeError {}

    pub fn parse_rfc3339_utc_datetime(value: &str) -> Result<DateTime<Utc>, ChronoTimeError> {
        super::parse_rfc3339_utc(value)
            .map_err(ChronoTimeError::InvalidRfc3339)
            .and_then(datetime_utc_to_chrono)
    }

    pub fn format_rfc3339_utc_datetime(value: &DateTime<Utc>) -> String {
        super::format_rfc3339_utc(&datetime_utc_from_chrono(value))
    }

    pub fn unix_seconds_to_chrono(value: i64) -> Result<DateTime<Utc>, ChronoTimeError> {
        DateTime::<Utc>::from_timestamp(value, 0).ok_or(ChronoTimeError::OutOfRange)
    }

    pub fn datetime_utc_to_chrono(value: DateTimeUtc) -> Result<DateTime<Utc>, ChronoTimeError> {
        DateTime::<Utc>::from_timestamp(value.unix_secs, value.nanos)
            .ok_or(ChronoTimeError::OutOfRange)
    }

    pub fn datetime_utc_from_chrono(value: &DateTime<Utc>) -> DateTimeUtc {
        DateTimeUtc {
            unix_secs: value.timestamp(),
            nanos: value.timestamp_subsec_nanos(),
        }
    }

    pub fn is_fresh_since(now: DateTime<Utc>, fetched_at: DateTime<Utc>, ttl: Duration) -> bool {
        if ttl.is_zero() {
            return false;
        }
        let Ok(ttl_duration) = ChronoDuration::from_std(ttl) else {
            return false;
        };
        now.signed_duration_since(fetched_at) <= ttl_duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_rfc3339_offsets_to_utc() {
        let timestamp = parse_rfc3339_utc("2030-01-01T07:00:00+07:00").unwrap();
        assert_eq!(timestamp.unix_secs, 1_893_456_000);
        assert_eq!(format_rfc3339_utc(&timestamp), "2030-01-01T00:00:00Z");
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn converts_unix_seconds_to_chrono_utc() {
        let timestamp = crate::chrono::unix_seconds_to_chrono(1_893_456_000).unwrap();
        assert_eq!(
            crate::chrono::format_rfc3339_utc_datetime(&timestamp),
            "2030-01-01T00:00:00Z"
        );
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn checks_chrono_ttl_freshness() {
        let fetched_at = crate::chrono::unix_seconds_to_chrono(100).unwrap();
        let now = crate::chrono::unix_seconds_to_chrono(130).unwrap();
        assert!(crate::chrono::is_fresh_since(
            now,
            fetched_at,
            core::time::Duration::from_secs(30)
        ));
        assert!(!crate::chrono::is_fresh_since(
            now,
            fetched_at,
            core::time::Duration::from_secs(29)
        ));
    }
}
