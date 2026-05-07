//! HTTP CONNECT and SOCKS5 protocol helpers.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::str;
use edgerun_encoding::byteorder::read_u16_be;

pub const SOCKS5_VERSION: u8 = 0x05;
pub const SOCKS5_CMD_CONNECT: u8 = 0x01;
pub const SOCKS5_ATYP_IPV4: u8 = 0x01;
pub const SOCKS5_ATYP_DOMAIN: u8 = 0x03;
pub const SOCKS5_ATYP_IPV6: u8 = 0x04;
pub const SOCKS5_REP_SUCCESS: u8 = 0x00;
pub const SOCKS5_REP_GENERAL_FAILURE: u8 = 0x01;
pub const SOCKS5_REP_CONN_REFUSED: u8 = 0x05;
pub const SOCKS5_REP_TTL_EXPIRED: u8 = 0x06;
pub const SOCKS5_REP_CMD_NOT_SUPPORTED: u8 = 0x07;
pub const SOCKS5_REP_ADDR_NOT_SUPPORTED: u8 = 0x08;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProxyError {
    Empty,
    InvalidUtf8,
    BadRequest,
    UnsupportedVersion,
    UnsupportedCommand,
    UnsupportedAddressType,
    NoAcceptableAuth,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HttpProxyRequest {
    Connect { target: String },
    Forward { method: String, path: String },
}

pub fn parse_http_proxy_request(bytes: &[u8]) -> Result<HttpProxyRequest, ProxyError> {
    let request = str::from_utf8(bytes).map_err(|_| ProxyError::InvalidUtf8)?;
    let first_line = request.lines().next().ok_or(ProxyError::Empty)?;
    let mut parts = first_line.split_whitespace();
    let method = parts.next().ok_or(ProxyError::BadRequest)?;
    let target = parts.next().ok_or(ProxyError::BadRequest)?;
    if method == "CONNECT" {
        Ok(HttpProxyRequest::Connect {
            target: target.to_string(),
        })
    } else {
        Ok(HttpProxyRequest::Forward {
            method: method.to_string(),
            path: target.to_string(),
        })
    }
}

pub fn split_host_port(target: &str, default_port: u16) -> (&str, u16) {
    let parts: Vec<&str> = target.split(':').collect();
    let host = *parts.first().unwrap_or(&target);
    let port = parts
        .get(1)
        .and_then(|part| part.parse().ok())
        .unwrap_or(default_port);
    (host, port)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Socks5Request {
    Connect { host: String, port: u16 },
}

pub fn socks5_select_no_auth(greeting: &[u8]) -> Result<[u8; 2], ProxyError> {
    if greeting.len() < 3 || greeting[0] != SOCKS5_VERSION {
        return Err(ProxyError::UnsupportedVersion);
    }
    let method_count = greeting[1] as usize;
    let methods = greeting
        .get(2..2 + method_count)
        .ok_or(ProxyError::BadRequest)?;
    if methods.contains(&0x00) {
        Ok([SOCKS5_VERSION, 0x00])
    } else {
        Err(ProxyError::NoAcceptableAuth)
    }
}

pub fn parse_socks5_request(bytes: &[u8]) -> Result<Socks5Request, ProxyError> {
    if bytes.len() < 5 || bytes[0] != SOCKS5_VERSION {
        return Err(ProxyError::UnsupportedVersion);
    }
    if bytes[1] != SOCKS5_CMD_CONNECT {
        return Err(ProxyError::UnsupportedCommand);
    }
    if bytes[2] != 0 {
        return Err(ProxyError::BadRequest);
    }
    match bytes[3] {
        SOCKS5_ATYP_IPV4 if bytes.len() >= 10 => {
            let host = alloc::format!("{}.{}.{}.{}", bytes[4], bytes[5], bytes[6], bytes[7]);
            let port = read_u16_be(bytes, 8);
            Ok(Socks5Request::Connect { host, port })
        }
        SOCKS5_ATYP_DOMAIN => {
            let len = bytes[4] as usize;
            if bytes.len() < 5 + len + 2 {
                return Err(ProxyError::BadRequest);
            }
            let host = str::from_utf8(&bytes[5..5 + len])
                .map_err(|_| ProxyError::InvalidUtf8)?
                .to_string();
            let port = read_u16_be(bytes, 5 + len);
            Ok(Socks5Request::Connect { host, port })
        }
        _ => Err(ProxyError::UnsupportedAddressType),
    }
}

pub fn socks5_reply(rep: u8) -> [u8; 10] {
    [SOCKS5_VERSION, rep, 0, SOCKS5_ATYP_IPV4, 0, 0, 0, 0, 0, 0]
}
