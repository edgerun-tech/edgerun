//! RFC 2822 date formatting.
//!
//! Produces date strings in the format `Day, DD Mon YYYY HH:MM:SS +0000`,
//! suitable for email `Date:` headers.
//!
//! # Examples
//! ```
//! use edgerun_encoding::rfc2822::format_rfc2822_utc;
//! // 2030-01-01 00:00:00 UTC is a Tuesday
//! let s = format_rfc2822_utc(1893456000);
//! assert_eq!(s, "Tue, 01 Jan 2030 00:00:00 +0000");
//! ```

use alloc::format;
use alloc::string::String;

const MONTHS: &[&str] = &[
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const DAYS: &[&str] = &["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Format a unix timestamp (seconds since epoch) as an RFC 2822 date in UTC.
///
/// Output format: `Day, DD Mon YYYY HH:MM:SS +0000`
pub fn format_rfc2822_utc(unix_secs: u64) -> String {
    let days = unix_secs / 86400;

    // Compute year (approximate, then refine)
    let mut year = 1970 + days / 365;
    let mut doy = days as i64 - (year - 1970) as i64 * 365;
    // Subtract leap days
    let leap_days = count_leap_days(1970, year) as i64;
    doy = doy.saturating_sub(leap_days);

    // Refine: if doy < 0, we overshot the year
    while doy >= days_in_year(year) as i64 {
        doy -= days_in_year(year) as i64;
        year += 1;
    }
    while doy < 0 {
        year -= 1;
        doy += days_in_year(year) as i64;
    }

    // Month and day
    let is_leap = is_leap_year(year);
    let month_days: [u32; 12] = [
        31,
        if is_leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0usize;
    let mut day = doy;
    for (i, &md) in month_days.iter().enumerate() {
        if day < md as i64 {
            month = i;
            break;
        }
        day -= md as i64;
    }
    day += 1; // 1-based

    let time_of_day = unix_secs % 86400;
    let hours = (time_of_day / 3600) as u32;
    let mins = ((time_of_day % 3600) / 60) as u32;
    let secs = (time_of_day % 60) as u32;
    let dow = ((days + 3) % 7) as usize; // 1970-01-01 was Thursday (index 3)

    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} +0000",
        DAYS[dow], day, MONTHS[month], year, hours, mins, secs
    )
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn days_in_year(year: u64) -> u64 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

/// Count leap years in range [start, end) (exclusive of end).
fn count_leap_days(start: u64, end: u64) -> u64 {
    if end <= start {
        return 0;
    }
    // Count leap years in [0, n): years divisible by 4, minus by 100, plus by 400
    let count_leaps = |n: u64| -> u64 {
        if n == 0 {
            return 0;
        }
        let n = n - 1; // up to and including year n-1
        n / 4 - n / 100 + n / 400
    };
    count_leaps(end) - count_leaps(start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_date_tuesday() {
        // 2030-01-01 00:00:00 UTC = Tuesday
        let s = format_rfc2822_utc(1893456000);
        assert_eq!(s, "Tue, 01 Jan 2030 00:00:00 +0000");
    }

    #[test]
    fn epoch_date() {
        // 1970-01-01 00:00:00 UTC = Thursday
        let s = format_rfc2822_utc(0);
        assert_eq!(s, "Thu, 01 Jan 1970 00:00:00 +0000");
    }

    #[test]
    fn leap_year_date() {
        // 2020-02-29 12:00:00 UTC = Saturday
        // Unix timestamp for 2020-02-29T12:00:00Z
        let s = format_rfc2822_utc(1582977600);
        assert_eq!(s, "Sat, 29 Feb 2020 12:00:00 +0000");
    }

    #[test]
    fn end_of_year() {
        // 2025-12-31 23:59:59 UTC = Wednesday
        let s = format_rfc2822_utc(1767225599);
        assert_eq!(s, "Wed, 31 Dec 2025 23:59:59 +0000");
    }
}
