//! Async DNS client — sends queries and parses responses over UDP.
//!
//! Uses the edgerun-rt async runtime for non-blocking I/O.

use alloc::{boxed::Box, format, string::{String, ToString}, vec, vec::Vec};
use crate::std::io;
use crate::std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use core::pin::Pin;
use crate::std::time::Duration;

use crate::compat::AsyncUdpSocket;

use super::message::{DnsMessage, DnsResponseCode};
use super::record::{DnsRecordData, DnsRecordType};

/// Async DNS client for sending queries.
///
/// # Example
/// ```no_run
/// use edgerun_dns::DnsClient;
/// use edgerun_dns::record::DnsRecordType;
/// use crate::compat::Runtime;
///
/// let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
/// rt.block_on(async {
///     let mut client = DnsClient::new("8.8.8.8:53").unwrap();
///     match client.query_a("www.example.com").await {
///         Ok(ips) => println!("IPs: {:?}", ips),
///         Err(e) => eprintln!("DNS query failed: {}", e),
///     }
/// });
/// ```
pub struct DnsClient {
    socket: AsyncUdpSocket,
    server: SocketAddr,
    next_id: u16,
    timeout: Duration,
}

impl DnsClient {
    /// Create a new DNS client targeting the given server.
    ///
    /// `server_addr` is e.g. `"8.8.8.8:53"` or `"127.0.0.1:53"`.
    pub fn new(server_addr: &str) -> Result<Self, io::Error> {
        let server: SocketAddr = server_addr.parse().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid server address: {}", server_addr),
            )
        })?;

        let socket = AsyncUdpSocket::bind("0.0.0.0:0")?;

        Ok(Self {
            socket,
            server,
            next_id: 0x1000,
            timeout: Duration::from_secs(5),
        })
    }

    /// Create a DNS client using system nameservers from `/etc/resolv.conf`.
    ///
    /// Returns `None` if resolv.conf is missing/empty and no fallback is available.
    pub fn system() -> Option<Self> {
        let conf = crate::resolv_conf::ResolvConf::load();
        conf.server_addrs()
            .first()
            .and_then(|addr| Self::new(addr).ok())
    }

    /// Set the query timeout.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Query for A records (IPv4 addresses).
    pub async fn query_a(&mut self, name: &str) -> Result<Vec<Ipv4Addr>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::A).await?;

        let mut ips = Vec::new();
        for answer in &msg.answers {
            if answer.rtype == DnsRecordType::A {
                if let DnsRecordData::A(ip) = &answer.data {
                    ips.push(*ip);
                }
            }
        }
        Ok(ips)
    }

    /// Query for AAAA records (IPv6 addresses).
    pub async fn query_aaaa(&mut self, name: &str) -> Result<Vec<Ipv6Addr>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::AAAA).await?;

        let mut ips = Vec::new();
        for answer in &msg.answers {
            if answer.rtype == DnsRecordType::AAAA {
                if let DnsRecordData::AAAA(ip) = &answer.data {
                    ips.push(*ip);
                }
            }
        }
        Ok(ips)
    }

    /// Query for CNAME records.
    pub async fn query_cname(&mut self, name: &str) -> Result<Vec<String>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::CNAME).await?;

        let mut cnames = Vec::new();
        for answer in &msg.answers {
            if answer.rtype == DnsRecordType::CNAME {
                if let DnsRecordData::CNAME(target) = &answer.data {
                    cnames.push(target.clone());
                }
            }
        }
        Ok(cnames)
    }

    /// Query for MX records.
    pub async fn query_mx(&mut self, name: &str) -> Result<Vec<(u16, String)>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::MX).await?;

        let mut mx = Vec::new();
        for answer in &msg.answers {
            if answer.rtype == DnsRecordType::MX {
                if let DnsRecordData::MX { priority, exchange } = &answer.data {
                    mx.push((*priority, exchange.clone()));
                }
            }
        }
        Ok(mx)
    }

    /// Query for TXT records.
    pub async fn query_txt(&mut self, name: &str) -> Result<Vec<String>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::TXT).await?;

        let mut txts = Vec::new();
        for answer in &msg.answers {
            if answer.rtype == DnsRecordType::TXT {
                if let DnsRecordData::TXT(text) = &answer.data {
                    txts.push(text.clone());
                }
            }
        }
        Ok(txts)
    }

    /// Query for NS records.
    pub async fn query_ns(&mut self, name: &str) -> Result<Vec<String>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::NS).await?;

        let mut ns = Vec::new();
        for answer in &msg.answers {
            if answer.rtype == DnsRecordType::NS {
                if let DnsRecordData::NS(nameserver) = &answer.data {
                    ns.push(nameserver.clone());
                }
            }
        }
        Ok(ns)
    }

    /// Query for PTR records (reverse DNS).
    pub async fn query_ptr(&mut self, name: &str) -> Result<Vec<String>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::PTR).await?;

        let mut ptrs = Vec::new();
        for answer in &msg.answers {
            if answer.rtype == DnsRecordType::PTR {
                if let DnsRecordData::PTR(ptr_name) = &answer.data {
                    ptrs.push(ptr_name.clone());
                }
            }
        }
        Ok(ptrs)
    }

    /// Query for HINFO records.
    pub async fn query_hinfo(&mut self, name: &str) -> Result<Vec<(String, String)>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::HINFO).await?;
        let mut results = Vec::new();
        for answer in &msg.answers {
            if let DnsRecordData::HINFO { cpu, os } = &answer.data {
                results.push((cpu.clone(), os.clone()));
            }
        }
        Ok(results)
    }

    /// Query for URI records.
    pub async fn query_uri(&mut self, name: &str) -> Result<Vec<(u16, u16, String)>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::URI).await?;
        let mut results = Vec::new();
        for answer in &msg.answers {
            if let DnsRecordData::URI {
                priority,
                weight,
                target,
            } = &answer.data
            {
                results.push((*priority, *weight, target.clone()));
            }
        }
        Ok(results)
    }

    /// Send a generic query and return the full response message.
    pub async fn query(
        &mut self,
        name: &str,
        qtype: DnsRecordType,
    ) -> Result<DnsMessage, io::Error> {
        self.send_query(name, qtype).await
    }

    // --- Internal ---

    async fn send_query(
        &mut self,
        name: &str,
        qtype: DnsRecordType,
    ) -> Result<DnsMessage, io::Error> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let msg = DnsMessage::query(id, name.to_string(), qtype);
        let wire = msg.to_wire();

        // Send the query — async, non-blocking.
        poll_send_to(&mut self.socket, &wire, self.server).await?;

        // Read response with timeout — async, non-blocking.
        let response = crate::compat::timeout(self.timeout, poll_recv(&mut self.socket))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "DNS query timed out"))??;

        let response = DnsMessage::from_wire(&response)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if response.header.id != id {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DNS response ID mismatch",
            ));
        }

        if response.header.response_code != DnsResponseCode::NoError {
            return Err(io::Error::other(format!(
                "DNS error: {}",
                response.header.response_code.as_str()
            )));
        }

        Ok(response)
    }
}

// ---------------------------------------------------------------------------
// Async helpers
// ---------------------------------------------------------------------------

async fn poll_send_to(
    socket: &mut AsyncUdpSocket,
    buf: &[u8],
    target: SocketAddr,
) -> io::Result<usize> {
    use core::future::poll_fn;
    poll_fn(|cx| Pin::new(&mut *socket).poll_send_to(cx, buf, target)).await
}

async fn poll_recv(socket: &mut AsyncUdpSocket) -> io::Result<Vec<u8>> {
    use core::future::poll_fn;
    let mut buf = [0u8; 4096];
    let (n, _src) = poll_fn(|cx| Pin::new(&mut *socket).poll_recv_from(cx, &mut buf)).await?;
    Ok(buf[..n].to_vec())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = DnsClient::new("8.8.8.8:53");
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_invalid_address() {
        let client = DnsClient::new("not-an-address");
        assert!(client.is_err());
    }

    #[test]
    fn test_query_build() {
        use super::super::message::DnsMessage;
        // Just verify the query message builds correctly
        let msg = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::A);
        let wire = msg.to_wire();
        assert!(wire.len() > 12);
    }
}
