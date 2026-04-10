//! QPACK static table (RFC 9204 Appendix A)
//! Contains 99 common header fields

/// Static table entry
pub type StaticEntry = (&'static str, &'static str);

/// QPACK static table (99 entries)
pub const STATIC_TABLE: &[StaticEntry] = &[
    ("", ""),                             // 0 (unused)
    (":authority", ""),                   // 1
    (":path", "/"),                       // 2
    ("age", "0"),                         // 3
    ("content-disposition", ""),          // 4
    ("content-length", "0"),              // 5
    ("cookie", ""),                       // 6
    ("date", ""),                         // 7
    ("etag", ""),                         // 8
    ("if-modified-since", ""),            // 9
    ("if-none-match", ""),                // 10
    ("last-modified", ""),                // 11
    ("link", ""),                         // 12
    ("location", ""),                     // 13
    ("referer", ""),                      // 14
    ("set-cookie", ""),                   // 15
    (":method", "CONNECT"),               // 16
    (":method", "DELETE"),                // 17
    (":method", "GET"),                   // 18
    (":method", "HEAD"),                  // 19
    (":method", "OPTIONS"),               // 20
    (":method", "POST"),                  // 21
    (":method", "PUT"),                   // 22
    (":path", "/index.html"),             // 23
    (":scheme", "ftp"),                   // 24
    (":scheme", "http"),                  // 25
    (":scheme", "https"),                 // 26
    (":status", "103"),                   // 27
    (":status", "200"),                   // 28
    (":status", "304"),                   // 29
    (":status", "404"),                   // 30
    (":status", "503"),                   // 31
    ("accept", "*/*"),                    // 32
    ("accept", "application/dns-message"),// 33
    ("accept-encoding", "gzip, deflate, br"), // 34
    ("accept-ranges", "bytes"),           // 35
    ("access-control-allow-headers", "cache-control"), // 36
    ("access-control-allow-origin", "*"), // 37
    ("cache-control", "max-age=0"),       // 38
    ("cache-control", "no-cache"),        // 39
    ("cache-control", "no-store"),        // 40
    ("cache-control", "public, max-age=31536000"), // 41
    ("content-encoding", "br"),           // 42
    ("content-encoding", "gzip"),         // 43
    ("content-type", "application/dns-message"), // 44
    ("content-type", "application/javascript"), // 45
    ("content-type", "application/json"), // 46
    ("content-type", "application/x-www-form-urlencoded"), // 47
    ("content-type", "image/gif"),        // 48
    ("content-type", "image/jpeg"),       // 49
    ("content-type", "image/png"),        // 50
    ("content-type", "text/css"),         // 51
    ("content-type", "text/html;charset=utf-8"), // 52
    ("content-type", "text/plain"),       // 53
    ("content-type", "text/plain;charset=utf-8"), // 54
    ("range", "bytes=0-"),                // 55
    ("strict-transport-security", "max-age=31536000"), // 56
    ("strict-transport-security", "max-age=31536000; includesubdomains"), // 57
    ("strict-transport-security", "max-age=31536000; includesubdomains; preload"), // 58
    ("vary", "accept-encoding"),          // 59
    ("vary", "origin"),                   // 60
    ("x-content-type-options", "nosniff"),// 61
    ("x-xss-protection", "1; mode=block"),// 62
    (":status", "100"),                   // 63
    (":status", "204"),                   // 64
    (":status", "206"),                   // 65
    (":status", "302"),                   // 66
    (":status", "400"),                   // 67
    (":status", "403"),                   // 68
    (":status", "421"),                   // 69
    (":status", "425"),                   // 70
    (":status", "500"),                   // 71
    ("accept-language", ""),              // 72
    ("access-control-allow-headers", "*"),// 73
    ("access-control-allow-origin", "null"), // 74
    ("access-control-expose-headers", "content-length"), // 75
    ("access-control-request-headers", "content-type"), // 76
    ("access-control-request-method", "get"), // 77
    ("access-control-request-method", "post"), // 78
    ("alt-svc", "clear"),                 // 79
    ("authorization", ""),                // 80
    ("content-encoding", "identity"),     // 81
    ("content-type", "application/octet-stream"), // 82
    ("early-data", "1"),                  // 83
    ("expect-ct", "max-age=0"),           // 84
    ("forwarded", ""),                    // 85
    ("if-range", ""),                     // 86
    ("origin", ""),                       // 87
    ("purpose", "prefetch"),              // 88
    ("server", ""),                       // 89
    ("timing-allow-origin", "*"),         // 90
    ("upgrade-insecure-requests", "1"),   // 91
    ("user-agent", ""),                   // 92
    ("x-forwarded-for", ""),              // 93
    ("x-frame-options", "deny"),          // 94
    ("x-frame-options", "sameorigin"),    // 95
    (":status", "101"),                   // 96
    (":status", "201"),                   // 97
    (":status", "301"),                   // 98
    (":status", "303"),                   // 99
];

/// Get static table entry by index
pub fn get_static_entry(index: usize) -> Option<&'static StaticEntry> {
    if index > 0 && index < STATIC_TABLE.len() {
        Some(&STATIC_TABLE[index])
    } else {
        None
    }
}

/// Find entry in static table by name
pub fn find_by_name(name: &str) -> Option<(usize, &'static str)> {
    for (i, &(n, v)) in STATIC_TABLE.iter().enumerate() {
        if i == 0 {
            continue;
        }
        if n.eq_ignore_ascii_case(name) {
            return Some((i, v));
        }
    }
    None
}

/// Find entry in static table by name and value
pub fn find_by_name_value(name: &str, value: &str) -> Option<usize> {
    for (i, &(n, v)) in STATIC_TABLE.iter().enumerate() {
        if i == 0 {
            continue;
        }
        if n.eq_ignore_ascii_case(name) && v == value {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_table_size() {
        assert_eq!(STATIC_TABLE.len(), 100); // 99 entries + index 0
    }

    #[test]
    fn test_get_static_entry() {
        assert!(get_static_entry(0).is_none());
        let entry = get_static_entry(18).unwrap();
        assert_eq!(entry.0, ":method");
        assert_eq!(entry.1, "GET");

        let entry = get_static_entry(28).unwrap();
        assert_eq!(entry.0, ":status");
        assert_eq!(entry.1, "200");
    }

    #[test]
    fn test_find_by_name() {
        let (idx, value) = find_by_name(":authority").unwrap();
        assert_eq!(idx, 1);
        assert_eq!(value, "");

        let (idx, value) = find_by_name("content-type").unwrap();
        assert!(idx >= 44);
        assert!(!value.is_empty());
    }

    #[test]
    fn test_find_by_name_value() {
        let idx = find_by_name_value(":method", "GET").unwrap();
        assert_eq!(idx, 18);

        let idx = find_by_name_value(":status", "200").unwrap();
        assert_eq!(idx, 28);

        assert!(find_by_name_value(":method", "INVALID").is_none());
    }
}
