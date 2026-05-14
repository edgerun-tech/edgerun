#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

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
    use alloc::format;
    use alloc::string::String;
    use alloc::string::ToString;
    use core::marker::PhantomData;
    use core::ops::{Add, Sub};
    use core::time::Duration;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct ChronoUtc;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct ChronoLocal;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct ChronoDuration {
        secs: i64,
        nanos: i32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct ChronoUtcDateTime {
        inner: DateTimeUtc,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct DateTime<Tz = ChronoUtc> {
        inner: DateTimeUtc,
        _tz: PhantomData<Tz>,
    }

    pub trait ChronoDatelike {
        fn day(&self) -> u32;
    }

    pub trait ChronoTimeZone {
        fn with_ymd_and_hms(
            &self,
            year: i32,
            month: u32,
            day: u32,
            hour: u32,
            minute: u32,
            second: u32,
        ) -> LocalResult<ChronoUtcDateTime>;

        fn timestamp_opt(&self, secs: i64, nanos: u32) -> LocalResult<ChronoUtcDateTime>;
    }

    pub struct LocalResult<T>(Option<T>);

    impl<T> LocalResult<T> {
        pub fn single(self) -> Option<T> {
            self.0
        }

        pub fn unwrap(self) -> T {
            self.0.unwrap()
        }
    }

    pub type ChronoUtcDateTimeAlias = ChronoUtcDateTime;

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

    pub fn parse_rfc3339_utc_datetime(value: &str) -> Result<ChronoUtcDateTime, ChronoTimeError> {
        super::parse_rfc3339_utc(value)
            .map_err(ChronoTimeError::InvalidRfc3339)
            .and_then(datetime_utc_to_chrono)
    }

    pub fn format_rfc3339_utc_datetime(value: &ChronoUtcDateTime) -> String {
        super::format_rfc3339_utc(&datetime_utc_from_chrono(value))
    }

    pub fn unix_seconds_to_chrono(value: i64) -> Result<ChronoUtcDateTime, ChronoTimeError> {
        ChronoUtc
            .timestamp_opt(value, 0)
            .single()
            .ok_or(ChronoTimeError::OutOfRange)
    }

    pub fn datetime_utc_to_chrono(
        value: DateTimeUtc,
    ) -> Result<ChronoUtcDateTime, ChronoTimeError> {
        Ok(ChronoUtcDateTime { inner: value })
    }

    pub fn datetime_utc_from_chrono(value: &ChronoUtcDateTime) -> DateTimeUtc {
        value.inner
    }

    pub fn is_fresh_since(
        now: ChronoUtcDateTime,
        fetched_at: ChronoUtcDateTime,
        ttl: Duration,
    ) -> bool {
        if ttl.is_zero() {
            return false;
        }
        let Ok(age) = now.signed_duration_since(fetched_at).to_std() else {
            return false;
        };
        age <= ttl
    }

    impl ChronoUtc {
        pub fn now() -> ChronoUtcDateTime {
            let duration = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            ChronoUtcDateTime {
                inner: DateTimeUtc {
                    unix_secs: duration.as_secs() as i64,
                    nanos: duration.subsec_nanos(),
                },
            }
        }
    }

    impl ChronoLocal {
        pub fn now() -> DateTime<ChronoLocal> {
            DateTime {
                inner: ChronoUtc::now().inner,
                _tz: PhantomData,
            }
        }
    }

    impl ChronoTimeZone for ChronoUtc {
        fn with_ymd_and_hms(
            &self,
            year: i32,
            month: u32,
            day: u32,
            hour: u32,
            minute: u32,
            second: u32,
        ) -> LocalResult<ChronoUtcDateTime> {
            if year < 1970
                || !(1..=12).contains(&month)
                || day == 0
                || day > days_in_month(year as u32, month)
                || hour > 23
                || minute > 59
                || second > 59
            {
                return LocalResult(None);
            }
            LocalResult(Some(ChronoUtcDateTime {
                inner: DateTimeUtc {
                    unix_secs: date_to_unix_secs(year as u32, month, day, hour, minute, second),
                    nanos: 0,
                },
            }))
        }

        fn timestamp_opt(&self, secs: i64, nanos: u32) -> LocalResult<ChronoUtcDateTime> {
            if nanos >= 1_000_000_000 {
                return LocalResult(None);
            }
            LocalResult(Some(ChronoUtcDateTime {
                inner: DateTimeUtc {
                    unix_secs: secs,
                    nanos,
                },
            }))
        }
    }

    impl ChronoDuration {
        pub fn seconds(value: i64) -> Self {
            Self {
                secs: value,
                nanos: 0,
            }
        }

        pub fn minutes(value: i64) -> Self {
            Self::seconds(value.saturating_mul(60))
        }

        pub fn hours(value: i64) -> Self {
            Self::minutes(value.saturating_mul(60))
        }

        pub fn days(value: i64) -> Self {
            Self::hours(value.saturating_mul(24))
        }

        pub fn from_std(value: Duration) -> Result<Self, ChronoTimeError> {
            Ok(Self {
                secs: i64::try_from(value.as_secs()).map_err(|_| ChronoTimeError::OutOfRange)?,
                nanos: value.subsec_nanos() as i32,
            })
        }

        pub fn to_std(self) -> Result<Duration, ChronoTimeError> {
            if self.secs < 0 || self.nanos < 0 {
                return Err(ChronoTimeError::OutOfRange);
            }
            Ok(Duration::new(self.secs as u64, self.nanos as u32))
        }
    }

    impl Add for ChronoDuration {
        type Output = Self;

        fn add(self, rhs: Self) -> Self::Output {
            Self {
                secs: self.secs.saturating_add(rhs.secs),
                nanos: self.nanos.saturating_add(rhs.nanos),
            }
            .normalized()
        }
    }

    impl ChronoDuration {
        fn normalized(mut self) -> Self {
            while self.nanos >= 1_000_000_000 {
                self.secs = self.secs.saturating_add(1);
                self.nanos -= 1_000_000_000;
            }
            while self.nanos < 0 {
                self.secs = self.secs.saturating_sub(1);
                self.nanos += 1_000_000_000;
            }
            self
        }
    }

    impl Add<ChronoDuration> for ChronoUtcDateTime {
        type Output = Self;

        fn add(self, rhs: ChronoDuration) -> Self::Output {
            Self {
                inner: add_duration(self.inner, rhs),
            }
        }
    }

    impl Sub<ChronoDuration> for ChronoUtcDateTime {
        type Output = Self;

        fn sub(self, rhs: ChronoDuration) -> Self::Output {
            Self {
                inner: add_duration(
                    self.inner,
                    ChronoDuration {
                        secs: rhs.secs.saturating_neg(),
                        nanos: rhs.nanos.saturating_neg(),
                    },
                ),
            }
        }
    }

    impl ChronoUtcDateTime {
        pub fn timestamp(&self) -> i64 {
            self.inner.unix_secs
        }

        pub fn timestamp_subsec_nanos(&self) -> u32 {
            self.inner.nanos
        }

        pub fn signed_duration_since(self, earlier: Self) -> ChronoDuration {
            let mut secs = self.inner.unix_secs.saturating_sub(earlier.inner.unix_secs);
            let mut nanos = self.inner.nanos as i32 - earlier.inner.nanos as i32;
            if nanos < 0 {
                secs = secs.saturating_sub(1);
                nanos += 1_000_000_000;
            }
            ChronoDuration { secs, nanos }
        }

        pub fn with_timezone(&self, _tz: &ChronoLocal) -> DateTime<ChronoLocal> {
            DateTime {
                inner: self.inner,
                _tz: PhantomData,
            }
        }

        pub fn format(&self, pattern: &str) -> FormattedDateTime {
            FormattedDateTime {
                inner: self.inner,
                pattern: pattern.to_string(),
            }
        }

        pub fn to_rfc3339(&self) -> String {
            super::format_rfc3339_utc(&self.inner)
        }
    }

    impl core::fmt::Display for ChronoUtcDateTime {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str(&self.to_rfc3339())
        }
    }

    impl<Tz> DateTime<Tz> {
        pub fn timestamp(&self) -> i64 {
            self.inner.unix_secs
        }

        pub fn to_rfc3339(&self) -> String {
            super::format_rfc3339_utc(&self.inner)
        }

        pub fn format(&self, pattern: &str) -> FormattedDateTime {
            FormattedDateTime {
                inner: self.inner,
                pattern: pattern.to_string(),
            }
        }

        pub fn date_naive(&self) -> DateNaive {
            let (year, month, day, _, _, _) = unix_secs_to_parts(self.inner.unix_secs);
            DateNaive { year, month, day }
        }
    }

    impl<Tz> ChronoDatelike for DateTime<Tz> {
        fn day(&self) -> u32 {
            self.date_naive().day
        }
    }

    impl<Tz> core::fmt::Display for DateTime<Tz> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str(&self.to_rfc3339())
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DateNaive {
        year: u32,
        month: u32,
        day: u32,
    }

    pub struct FormattedDateTime {
        inner: DateTimeUtc,
        pattern: String,
    }

    impl core::fmt::Display for FormattedDateTime {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let (year, month, day, hour, minute, _second) =
                unix_secs_to_parts(self.inner.unix_secs);
            let mut text = self.pattern.clone();
            text = text.replace("%Y", &format!("{year:04}"));
            text = text.replace("%m", &format!("{month:02}"));
            text = text.replace("%d", &format!("{day:02}"));
            text = text.replace("%-d", &day.to_string());
            text = text.replace("%b", month_name(month));
            let (hour12, suffix) = if hour == 0 {
                (12, "AM")
            } else if hour < 12 {
                (hour, "AM")
            } else if hour == 12 {
                (12, "PM")
            } else {
                (hour - 12, "PM")
            };
            text = text.replace("%-I", &hour12.to_string());
            text = text.replace("%M", &format!("{minute:02}"));
            text = text.replace("%p", suffix);
            f.write_str(&text)
        }
    }

    fn add_duration(value: DateTimeUtc, duration: ChronoDuration) -> DateTimeUtc {
        let normalized = duration.normalized();
        let mut secs = value.unix_secs.saturating_add(normalized.secs);
        let mut nanos = value.nanos as i32 + normalized.nanos;
        if nanos >= 1_000_000_000 {
            secs = secs.saturating_add(1);
            nanos -= 1_000_000_000;
        } else if nanos < 0 {
            secs = secs.saturating_sub(1);
            nanos += 1_000_000_000;
        }
        DateTimeUtc {
            unix_secs: secs,
            nanos: nanos as u32,
        }
    }

    fn is_leap_year(year: u32) -> bool {
        (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
    }

    fn days_in_month(year: u32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if is_leap_year(year) {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        }
    }

    fn date_to_unix_secs(
        year: u32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> i64 {
        let mut days: i64 = 0;
        for y in 1970..year {
            days += if is_leap_year(y) { 366 } else { 365 };
        }
        for m in 1..month {
            days += days_in_month(year, m) as i64;
        }
        days += (day - 1) as i64;
        days * 86_400 + hour as i64 * 3_600 + minute as i64 * 60 + second as i64
    }

    fn unix_secs_to_parts(unix_secs: i64) -> (u32, u32, u32, u32, u32, u32) {
        let mut remaining = unix_secs.max(0);
        let mut year = 1970u32;
        loop {
            let days = if is_leap_year(year) { 366 } else { 365 };
            let secs = days as i64 * 86_400;
            if remaining < secs {
                break;
            }
            remaining -= secs;
            year += 1;
        }
        let mut month = 1u32;
        loop {
            let secs = days_in_month(year, month) as i64 * 86_400;
            if remaining < secs {
                break;
            }
            remaining -= secs;
            month += 1;
        }
        let day = (remaining / 86_400) as u32 + 1;
        let rem = remaining % 86_400;
        let hour = (rem / 3_600) as u32;
        let minute = ((rem % 3_600) / 60) as u32;
        let second = (rem % 60) as u32;
        (year, month, day, hour, minute, second)
    }

    fn month_name(month: u32) -> &'static str {
        match month {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "",
        }
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
