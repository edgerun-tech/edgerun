//! HTTP semantics conformance tests based on RFC 9110, RFC 9112, and RFC 3986.
//!
//! These tests verify that core HTTP types correctly enforce the rules specified
//! in the relevant RFCs.

use crate::header::{HeaderMap, HeaderName, HeaderValue};
use crate::http1::request::Request;
use crate::http1::response::Response;
use crate::method::Method;
use crate::status::StatusCode;
use crate::uri::Uri;

// ---------------------------------------------------------------------------
// Method conformance (RFC 9110 Section 9)
// ---------------------------------------------------------------------------

#[test]
fn method_standard_methods_exist() {
    // RFC 9110 Section 9.1: GET, HEAD, POST, PUT, DELETE, CONNECT, OPTIONS, TRACE
    // Plus PATCH from RFC 5789
    let methods = [
        "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "CONNECT", "TRACE",
    ];
    for m in methods {
        let parsed: Method = m.parse().unwrap();
        assert_eq!(parsed.as_str(), m, "Method {m} should round-trip");
    }
}

#[test]
fn method_case_insensitive() {
    // RFC 9110 Section 9.1: method names are case-sensitive tokens, but all
    // registered methods are uppercase. Implementations should accept case-insensitive.
    let cases = [
        ("get", Method::GET),
        ("Get", Method::GET),
        ("post", Method::POST),
        ("Post", Method::POST),
        ("delete", Method::DELETE),
        ("Delete", Method::DELETE),
    ];
    for (input, expected) in cases {
        let parsed: Method = input.parse().unwrap();
        assert_eq!(
            parsed, expected,
            "Method parsing of '{input}' should be case-insensitive"
        );
    }
}

#[test]
fn method_rejects_invalid() {
    // Invalid methods: empty, contains non-token characters
    let invalid = [
        "",        // empty
        "GET ",    // trailing space (not a tchar)
        " POST",   // leading space
        "G E T",   // spaces between chars
        "foo bar", // space in middle
    ];
    for m in &invalid {
        assert!(
            m.parse::<Method>().is_err(),
            "Method '{m}' should be rejected"
        );
    }
}

#[test]
fn method_accepts_valid_token_as_extension() {
    // Any valid token (tchar+) that isn't a standard method becomes Extension
    let extension_tokens = [
        "get-post",     // hyphen is a tchar
        "123",          // digits are tchars
        "FOO-BAR_BAZ",  // mixed token chars
        "x-request-id", // common custom pattern
    ];
    for m in extension_tokens {
        let parsed = m.parse::<Method>().unwrap();
        assert!(
            parsed.is_extension(),
            "'{m}' should be parsed as extension method"
        );
    }
}

#[test]
fn method_extension_methods() {
    // RFC 9110 Section 9.1: any valid token is a method
    let extensions = [
        ("PROPFIND", Method::Extension("PROPFIND".to_string())), // WebDAV
        ("MKCOL", Method::Extension("MKCOL".to_string())),       // WebDAV
        ("COPY", Method::Extension("COPY".to_string())),         // WebDAV
        ("MOVE", Method::Extension("MOVE".to_string())),         // WebDAV
        ("LOCK", Method::Extension("LOCK".to_string())),         // WebDAV
        ("UNLOCK", Method::Extension("UNLOCK".to_string())),     // WebDAV
        ("SEARCH", Method::Extension("SEARCH".to_string())),     // RFC 5323
        ("CUSTOM", Method::Extension("CUSTOM".to_string())),     // Custom
        ("X-REQUEST", Method::Extension("X-REQUEST".to_string())), // Custom with hyphen
    ];
    for (input, expected) in extensions {
        let parsed: Method = input.parse().unwrap();
        assert_eq!(
            parsed, expected,
            "Method {input} should be parsed as extension"
        );
        assert!(parsed.is_extension());
        assert!(!parsed.is_standard());
    }
}

#[test]
fn method_extension_has_body() {
    // Extension methods may have bodies; we conservatively allow it
    assert!(Method::Extension("PROPFIND".to_string()).has_body());
    assert!(Method::Extension("CUSTOM".to_string()).has_body());
}

#[test]
fn method_extension_expects_response_body() {
    // Extension methods expect response bodies (unlike HEAD)
    assert!(Method::Extension("PROPFIND".to_string()).expects_response_body());
}

#[test]
fn method_has_body() {
    // RFC 9110: POST, PUT, PATCH can have a request body.
    // GET, HEAD, DELETE, OPTIONS, CONNECT, TRACE typically don't (though CONNECT tunnels data).
    assert!(Method::POST.has_body());
    assert!(Method::PUT.has_body());
    assert!(Method::PATCH.has_body());
    assert!(!Method::GET.has_body());
    assert!(!Method::HEAD.has_body());
    assert!(!Method::DELETE.has_body());
}

#[test]
fn method_expects_response_body() {
    // RFC 9110 Section 9.3.2: HEAD responses must not contain a body.
    assert!(!Method::HEAD.expects_response_body());
    assert!(Method::GET.expects_response_body());
    assert!(Method::POST.expects_response_body());
}

// ---------------------------------------------------------------------------
// Status Code conformance (RFC 9110 Section 15)
// ---------------------------------------------------------------------------

#[test]
fn status_code_valid_range() {
    // RFC 9110 Section 15: status codes are 3-digit numbers (100-599)
    assert!(StatusCode::new(100).is_ok());
    assert!(StatusCode::new(200).is_ok());
    assert!(StatusCode::new(301).is_ok());
    assert!(StatusCode::new(404).is_ok());
    assert!(StatusCode::new(500).is_ok());
    assert!(StatusCode::new(599).is_ok());
}

#[test]
fn status_code_rejects_out_of_range() {
    assert!(StatusCode::new(99).is_err());
    assert!(StatusCode::new(0).is_err());
    assert!(StatusCode::new(600).is_err());
    assert!(StatusCode::new(999).is_err());
}

#[test]
fn status_code_categories() {
    // RFC 9110 Section 15.1-15.5: Informational, Successful, Redirection, Client Error, Server Error
    let s100 = StatusCode::new(100).unwrap();
    assert!(s100.is_informational());
    assert!(!s100.is_success());

    let s200 = StatusCode::new(200).unwrap();
    assert!(s200.is_success());
    assert!(!s200.is_redirection());

    let s301 = StatusCode::new(301).unwrap();
    assert!(s301.is_redirection());

    let s404 = StatusCode::new(404).unwrap();
    assert!(s404.is_client_error());
    assert!(!s404.is_server_error());

    let s500 = StatusCode::new(500).unwrap();
    assert!(s500.is_server_error());
}

#[test]
fn status_code_reason_phrases() {
    // RFC 9110 Section 15: registered status codes have standard reason phrases
    assert_eq!(StatusCode::new(200).unwrap().reason(), "OK");
    assert_eq!(StatusCode::new(201).unwrap().reason(), "Created");
    assert_eq!(StatusCode::new(204).unwrap().reason(), "No Content");
    assert_eq!(StatusCode::new(301).unwrap().reason(), "Moved Permanently");
    assert_eq!(StatusCode::new(304).unwrap().reason(), "Not Modified");
    assert_eq!(StatusCode::new(400).unwrap().reason(), "Bad Request");
    assert_eq!(StatusCode::new(401).unwrap().reason(), "Unauthorized");
    assert_eq!(StatusCode::new(403).unwrap().reason(), "Forbidden");
    assert_eq!(StatusCode::new(404).unwrap().reason(), "Not Found");
    assert_eq!(
        StatusCode::new(500).unwrap().reason(),
        "Internal Server Error"
    );
    assert_eq!(StatusCode::new(502).unwrap().reason(), "Bad Gateway");
    assert_eq!(
        StatusCode::new(503).unwrap().reason(),
        "Service Unavailable"
    );
}

#[test]
fn status_code_unknown_reason() {
    // Unregistered codes in valid range should return "Unknown Status"
    // Note: 418 is registered by RFC 7168 (HTCPCP) as "I'm a teapot"
    assert_eq!(StatusCode::new(420).unwrap().reason(), "Unknown Status");
    assert_eq!(StatusCode::new(450).unwrap().reason(), "Unknown Status");
    assert_eq!(StatusCode::new(550).unwrap().reason(), "Unknown Status");
}

// ---------------------------------------------------------------------------
// HeaderName conformance (RFC 9110 Section 5.6.2 - Token)
// ---------------------------------------------------------------------------

#[test]
fn header_name_valid_characters() {
    // RFC 9110 Section 5.6.2: field-name = token
    // token = 1*tchar; tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." /
    //         "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
    let valid = [
        "content-type",
        "Content-Type",
        "x-custom-header",
        "X-Request-ID",
        "accept",
        "Accept-Encoding",
        "x-request-id_123",
    ];
    for name in valid {
        assert!(
            HeaderName::new(name.to_string()).is_ok(),
            "HeaderName '{name}' should be valid"
        );
    }
}

#[test]
fn header_name_rejects_invalid() {
    // RFC 9110 Section 5.6.2: field-name = token
    // token = 1*tchar where tchar = ALPHA / DIGIT / "!" / "#" / "$" / "%" / "&" / "'" / "*" /
    //   "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
    let invalid = [
        "",              // empty
        "content type",  // space
        "content\ttype", // tab (control char)
        "content:type",  // colon
        "content;type",  // semicolon
        "content/type",  // slash
        "content,type",  // comma
        "[bad]",         // brackets
        "<bad>",         // angle brackets
        "foo=bar",       // equals sign (not tchar)
        "foo?bar",       // question mark
        "foo@bar",       // at sign
        "foo[bar",       // square bracket
    ];
    for name in invalid {
        assert!(
            HeaderName::new(name.to_string()).is_err(),
            "HeaderName '{name}' should be rejected"
        );
    }
}

// ---------------------------------------------------------------------------
// HeaderValue conformance (RFC 9110 Section 5.5 - Field Values)
// ---------------------------------------------------------------------------

#[test]
fn header_value_valid() {
    // RFC 9110: field values can contain visible ASCII, space, and horizontal tab
    let valid = [
        "text/html",
        "application/json",
        "gzip, deflate, br",
        "Mozilla/5.0",
        "key=value",
        "hello\tworld", // tab is allowed
        "hello world",  // space is allowed
    ];
    for val in valid {
        assert!(
            HeaderValue::new(val.to_string()).is_ok(),
            "HeaderValue '{val}' should be valid"
        );
    }
}

#[test]
fn header_value_rejects_control_chars() {
    // RFC 9110 Section 5.5: field values allow visible ASCII (0x21-0x7E), SP (0x20), HTAB (0x09)
    // Control characters (except \t and space) are not allowed
    let invalid = [
        "hello\nworld",   // newline
        "hello\rworld",   // carriage return
        "hello\x00world", // null
        "hello\x1Bworld", // escape
        "hello\x7Fworld", // DEL (0x7F)
    ];
    for val in invalid {
        assert!(
            HeaderValue::new(val.to_string()).is_err(),
            "HeaderValue with invalid byte should be rejected: {val:?}"
        );
    }

    // Non-ASCII UTF-8 characters must also be rejected (RFC 9110 limits
    // header values to ASCII). These are valid UTF-8 strings with bytes >= 0x80.
    let non_ascii = [
        "hello🌍world", // emoji (multi-byte UTF-8)
        "café",         // non-ASCII Latin
        "日本語",       // CJK characters
    ];
    for val in non_ascii {
        assert!(
            HeaderValue::new(val.to_string()).is_err(),
            "HeaderValue with non-ASCII UTF-8 should be rejected: {val:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// HeaderMap conformance
// ---------------------------------------------------------------------------

#[test]
fn header_map_insert_and_get() {
    let mut map = HeaderMap::new();
    let _ = map.insert("content-type", "application/json");
    let _ = map.insert("accept", "text/html");

    assert!(map.contains_key("content-type"));
    assert!(map.contains_key("Content-Type")); // case-insensitive
    assert_eq!(
        map.get("content-type").unwrap().as_str(),
        "application/json"
    );
}

#[test]
fn header_map_multiple_values() {
    let mut map = HeaderMap::new();
    let _ = map.insert("accept", "text/html");
    let _ = map.insert("accept", "application/json");

    let all = map.get_all("accept");
    assert_eq!(all.len(), 2);
}

#[test]
fn header_map_empty() {
    let map = HeaderMap::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
    assert!(!map.contains_key("anything"));
}

// ---------------------------------------------------------------------------
// URI conformance (RFC 3986 + RFC 9112 for request targets)
// ---------------------------------------------------------------------------

#[test]
fn uri_absolute_http() {
    let uri: Uri = "http://example.com/path?query=1".parse().unwrap();
    assert!(uri.scheme().is_https() == false);
    assert_eq!(uri.host().unwrap(), "example.com");
    assert_eq!(uri.port().unwrap(), 80);
    assert_eq!(uri.path(), "/path");
    assert_eq!(uri.query().unwrap(), "query=1");
}

#[test]
fn uri_absolute_https() {
    let uri: Uri = "https://example.com:8443/api/v1".parse().unwrap();
    assert!(uri.scheme().is_https());
    assert_eq!(uri.host().unwrap(), "example.com");
    assert_eq!(uri.port().unwrap(), 8443);
    assert_eq!(uri.path(), "/api/v1");
    assert!(uri.query().is_none());
}

#[test]
fn uri_origin_form() {
    // RFC 9112: origin-form = absolute-path [ "?" query ]
    let uri: Uri = "/path/to/resource?key=value".parse().unwrap();
    assert_eq!(uri.path(), "/path/to/resource");
    assert_eq!(uri.query().unwrap(), "key=value");
    // Note: parser may set host from empty authority portion of origin-form URIs
}

#[test]
fn uri_request_target_origin_form() {
    let uri: Uri = "https://example.com/path?q=1#frag".parse().unwrap();
    // RFC 9112: fragment must NOT appear in request target
    let target = uri.request_target();
    assert_eq!(target, "/path?q=1");
    assert!(!target.contains('#'));
}

#[test]
fn uri_default_ports() {
    let http: Uri = "http://example.com/".parse().unwrap();
    assert_eq!(http.port().unwrap(), 80);

    let https: Uri = "https://example.com/".parse().unwrap();
    assert_eq!(https.port().unwrap(), 443);
}

#[test]
fn uri_explicit_port_overrides_default() {
    let uri: Uri = "http://example.com:8080/".parse().unwrap();
    assert_eq!(uri.port().unwrap(), 8080);
}

#[test]
fn uri_with_fragment() {
    let uri: Uri = "http://example.com/page#section1".parse().unwrap();
    assert_eq!(uri.fragment().unwrap(), "section1");
}

#[test]
fn uri_empty_path() {
    // When authority is present but no explicit path, RFC 3986 defaults to "/"
    let uri: Uri = "http://example.com".parse().unwrap();
    assert_eq!(uri.path(), "/");
}

#[test]
fn uri_with_userinfo_stripped() {
    // RFC 3986 allows userinfo but HTTP should not use it
    let uri: Uri = "http://user:pass@example.com/path".parse().unwrap();
    assert_eq!(uri.host().unwrap(), "example.com");
    assert!(uri.path() == "/path");
}

#[test]
fn uri_rejects_empty() {
    assert!("".parse::<Uri>().is_err());
}

// ---------------------------------------------------------------------------
// HTTP/1.1 Response parsing conformance (RFC 9112)
// ---------------------------------------------------------------------------

#[test]
fn response_parse_basic() {
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\nHello";
    let resp = Response::from_http(response).unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    assert!(resp.headers().contains_key("content-type"));
    assert_eq!(resp.body(), b"Hello");
}

#[test]
fn response_parse_status_codes() {
    let codes = [
        ("HTTP/1.1 200 OK\r\n\r\n", 200),
        ("HTTP/1.1 301 Moved\r\n\r\n", 301),
        ("HTTP/1.1 404 Not Found\r\n\r\n", 404),
        ("HTTP/1.1 500 Error\r\n\r\n", 500),
    ];
    for (raw, expected_code) in codes {
        let resp = Response::from_http(raw).unwrap();
        assert_eq!(resp.status().as_u16(), expected_code);
    }
}

#[test]
fn response_parse_multiple_headers() {
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 5\r\nServer: test\r\n\r\nHello";
    let resp = Response::from_http(response).unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    assert!(resp.headers().contains_key("content-type"));
    assert!(resp.headers().contains_key("content-length"));
    assert!(resp.headers().contains_key("server"));
}

#[test]
fn response_parse_no_body() {
    let response = "HTTP/1.1 204 No Content\r\n\r\n";
    let resp = Response::from_http(response).unwrap();
    assert_eq!(resp.status().as_u16(), 204);
    assert!(resp.body().is_empty());
}

#[test]
fn response_parse_with_body() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, world!";
    let resp = Response::from_http(response).unwrap();
    assert_eq!(resp.body_as_string().unwrap(), "Hello, world!");
}

#[test]
fn response_rejects_invalid_status_line() {
    let invalid = [
        "",                        // empty
        "Not HTTP",                // no status code
        "HTTP/1.1 abc OK\r\n\r\n", // non-numeric status
    ];
    for raw in invalid {
        assert!(
            Response::from_http(raw).is_err(),
            "Response should reject invalid status line: {raw:?}"
        );
    }
}

#[test]
fn response_parse_chunked_body() {
    // RFC 9112 §7.1: chunked body = chunk * chunk last-chunk trailer-part CRLF
    // chunk = chunk-size CRLF chunk-data CRLF
    let raw = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n7\r\nMozilla\r\n9\r\nDeveloper\r\n0\r\n\r\n";
    let resp = Response::from_http(raw).unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    assert_eq!(resp.body_as_string().unwrap(), "MozillaDeveloper");
}

#[test]
fn response_parse_chunked_body_single() {
    let raw =
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nd\r\nHello, world!\r\n0\r\n\r\n";
    let resp = Response::from_http(raw).unwrap();
    assert_eq!(resp.body_as_string().unwrap(), "Hello, world!");
}

#[test]
fn response_parse_chunked_body_multiple() {
    let raw = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nHello\r\n6\r\n world\r\n0\r\n\r\n";
    let resp = Response::from_http(raw).unwrap();
    assert_eq!(resp.body_as_string().unwrap(), "Hello world");
}

#[test]
fn response_parse_chunked_with_extensions() {
    // Chunk extensions after the hex size (RFC 9112 §7.1.1)
    let raw =
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5;foo=bar\r\nHello\r\n0\r\n\r\n";
    let resp = Response::from_http(raw).unwrap();
    assert_eq!(resp.body_as_string().unwrap(), "Hello");
}

#[test]
fn response_parse_content_length_overrides_raw_data() {
    // Content-Length should limit the body even if more data follows
    let raw = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\n12345EXTRA GARBAGE DATA";
    let resp = Response::from_http(raw).unwrap();
    assert_eq!(resp.body(), b"12345");
}

#[test]
fn response_no_body_1xx() {
    let raw = "HTTP/1.1 100 Continue\r\n\r\nThis should not be a body";
    let resp = Response::from_http(raw).unwrap();
    assert!(resp.body().is_empty());
}

#[test]
fn response_no_body_204() {
    let raw = "HTTP/1.1 204 No Content\r\n\r\nThis should not be a body";
    let resp = Response::from_http(raw).unwrap();
    assert!(resp.body().is_empty());
}

#[test]
fn response_no_body_304() {
    let raw = "HTTP/1.1 304 Not Modified\r\n\r\nThis should not be a body";
    let resp = Response::from_http(raw).unwrap();
    assert!(resp.body().is_empty());
}

#[test]
fn response_parse_trailer_headers() {
    // RFC 9112 §6.3: trailers follow the 0-length chunk, terminated by blank line
    let raw = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: X-Checksum\r\n\r\n5\r\nHello\r\n0\r\nX-Checksum: abc123\r\n\r\n";
    let resp = Response::from_http(raw).unwrap();
    assert_eq!(resp.body_as_string().unwrap(), "Hello");
    assert!(resp.trailers().contains_key("x-checksum"));
    assert_eq!(
        resp.trailers().get("x-checksum").unwrap().as_str(),
        "abc123"
    );
}

#[test]
fn response_no_trailers_without_chunked() {
    let raw = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\n12345";
    let resp = Response::from_http(raw).unwrap();
    assert!(resp.trailers().is_empty());
}

#[test]
fn response_chunked_no_trailers() {
    let raw = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nHello\r\n0\r\n\r\n";
    let resp = Response::from_http(raw).unwrap();
    assert_eq!(resp.body_as_string().unwrap(), "Hello");
    assert!(resp.trailers().is_empty());
}

// ---------------------------------------------------------------------------
// HTTP/1.1 Request parsing conformance (RFC 9112)
// ---------------------------------------------------------------------------

#[test]
fn request_parse_basic_get() {
    let raw = "GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let req = Request::from_http(raw).unwrap();
    assert!(matches!(req.method(), &Method::GET));
    assert_eq!(req.uri().path(), "/index.html");
    assert!(req.headers().contains_key("host"));
    assert!(req.body().is_none());
}

#[test]
fn request_parse_post_with_body() {
    let raw = "POST /api/data HTTP/1.1\r\nHost: example.com\r\nContent-Length: 13\r\nContent-Type: text/plain\r\n\r\nHello, world!";
    let req = Request::from_http(raw).unwrap();
    assert!(matches!(req.method(), &Method::POST));
    assert_eq!(req.uri().path(), "/api/data");
    assert_eq!(req.body().unwrap(), b"Hello, world!");
    assert!(req.headers().contains_key("content-type"));
}

#[test]
fn request_parse_all_methods() {
    let methods = [
        "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "CONNECT", "TRACE",
    ];
    for m in methods {
        let raw = format!("{m} / HTTP/1.1\r\nHost: x\r\n\r\n");
        let req = Request::from_http(&raw).unwrap();
        assert!(req.method().as_str() == m, "Method should be {m}");
    }
}

#[test]
fn request_parse_extension_method() {
    let raw = "PROPFIND /webdav/ HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let req = Request::from_http(raw).unwrap();
    assert!(req.method().is_extension());
    assert_eq!(req.method().as_str(), "PROPFIND");
}

#[test]
fn request_parse_multiple_headers() {
    let raw =
        "GET / HTTP/1.1\r\nHost: example.com\r\nAccept: text/html\r\nUser-Agent: Test\r\n\r\n";
    let req = Request::from_http(raw).unwrap();
    assert!(req.headers().contains_key("host"));
    assert!(req.headers().contains_key("accept"));
    assert!(req.headers().contains_key("user-agent"));
}

#[test]
fn request_parse_origin_form() {
    let raw = "GET /path/to/resource?key=value HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let req = Request::from_http(raw).unwrap();
    assert_eq!(req.uri().path(), "/path/to/resource");
    assert_eq!(req.uri().query().unwrap(), "key=value");
}

#[test]
fn request_parse_absolute_form() {
    let raw = "GET http://example.com:8080/path HTTP/1.1\r\n\r\n";
    let req = Request::from_http(raw).unwrap();
    assert_eq!(req.uri().host().unwrap(), "example.com");
    assert_eq!(req.uri().port().unwrap(), 8080);
    assert_eq!(req.uri().path(), "/path");
}

#[test]
fn request_parse_asterisk_form() {
    // RFC 9112: asterisk-form = "*" (used with OPTIONS)
    let raw = "OPTIONS * HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let req = Request::from_http(raw).unwrap();
    assert!(matches!(req.method(), &Method::OPTIONS));
    // The URI parser normalizes "*" to path "/". The asterisk is a special
    // request target that doesn't map to a standard URI path component.
    assert_eq!(req.uri().path(), "/");
}

#[test]
fn request_parse_body_respects_content_length() {
    let raw = "POST /api HTTP/1.1\r\nHost: x\r\nContent-Length: 5\r\n\r\n12345EXTRA GARBAGE";
    let req = Request::from_http(raw).unwrap();
    assert_eq!(req.body().unwrap(), b"12345");
}

#[test]
fn request_parse_no_body_without_content_length() {
    let raw = "GET / HTTP/1.1\r\nHost: x\r\n\r\n";
    let req = Request::from_http(raw).unwrap();
    assert!(req.body().is_none());
}

#[test]
fn request_rejects_invalid() {
    let invalid = [
        "",                        // empty
        "GET / HTTP/1.1\n",        // LF instead of CRLF (no \r\n found)
        "GET /\r\n",               // missing HTTP version
        "GET@ / HTTP/1.1\r\n\r\n", // @ is not a tchar in method
    ];
    for raw in invalid {
        assert!(
            Request::from_http(raw).is_err(),
            "Request should reject invalid input: {raw:?}"
        );
    }
}

#[test]
fn request_parse_chunked_body() {
    let raw = "POST /api HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: chunked\r\n\r\n7\r\nMozilla\r\n9\r\nDeveloper\r\n0\r\n\r\n";
    let req = Request::from_http(raw).unwrap();
    assert!(matches!(req.method(), &Method::POST));
    assert_eq!(req.body_as_str().unwrap(), "MozillaDeveloper");
}

#[test]
fn request_parse_chunked_body_single() {
    let raw = "PUT /data HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\nd\r\nHello, world!\r\n0\r\n\r\n";
    let req = Request::from_http(raw).unwrap();
    assert_eq!(req.body_as_str().unwrap(), "Hello, world!");
}

#[test]
fn request_rejects_invalid_chunked_body() {
    // Incomplete chunked body (missing final 0\r\n\r\n)
    let raw = "POST /api HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n7\r\nMozilla";
    assert!(Request::from_http(raw).is_err());
}
