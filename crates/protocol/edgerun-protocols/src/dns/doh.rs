//! DNS-over-HTTPS request helpers (RFC 8484).
//!
//! This module handles DoH request byte extraction and standard error
//! responses only. It does not own HTTP routing, TLS, sockets, or DNS lookup.

use alloc::vec::Vec;

use super::message::{DnsMessage, DnsResponseCode};

pub const DOH_DNS_MESSAGE_CONTENT_TYPE: &str = "application/dns-message";
pub const DOH_TEXT_CONTENT_TYPE: &str = "text/plain";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DohRequestError {
    GetDisabled,
    PostDisabled,
    InvalidDnsParameter,
}

pub fn decode_doh_get_query(dns_param: &str, enable_get: bool) -> Result<Vec<u8>, DohRequestError> {
    if !enable_get {
        return Err(DohRequestError::GetDisabled);
    }

    edgerun_encoding::base64::base64url_decode(dns_param)
        .map_err(|_| DohRequestError::InvalidDnsParameter)
}

pub fn decode_doh_post_query(body: &[u8], enable_post: bool) -> Result<Vec<u8>, DohRequestError> {
    if !enable_post {
        return Err(DohRequestError::PostDisabled);
    }
    Ok(body.to_vec())
}

pub fn doh_error_http_response(error: DohRequestError) -> (u16, &'static str, Vec<u8>) {
    match error {
        DohRequestError::GetDisabled => (
            405,
            DOH_TEXT_CONTENT_TYPE,
            b"GET method not enabled".to_vec(),
        ),
        DohRequestError::PostDisabled => (
            405,
            DOH_TEXT_CONTENT_TYPE,
            b"POST method not enabled".to_vec(),
        ),
        DohRequestError::InvalidDnsParameter => (
            400,
            DOH_TEXT_CONTENT_TYPE,
            b"Invalid dns parameter".to_vec(),
        ),
    }
}

pub fn doh_formerr_http_response() -> (u16, &'static str, Vec<u8>) {
    let resp = DnsMessage::response(0, DnsResponseCode::FormErr, Vec::new());
    (400, DOH_DNS_MESSAGE_CONTENT_TYPE, resp.to_wire())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_base64url_get_query() {
        let result = decode_doh_get_query("dGVzdA", true).unwrap();
        assert_eq!(result, b"test");
    }

    #[test]
    fn rejects_invalid_get_query() {
        assert_eq!(
            decode_doh_get_query("!!!invalid!!!", true).unwrap_err(),
            DohRequestError::InvalidDnsParameter
        );
    }

    #[test]
    fn post_query_is_wire_body() {
        assert_eq!(
            decode_doh_post_query(b"\x12\x34", true).unwrap(),
            b"\x12\x34"
        );
    }
}
