//! DNS client — sends queries and parses responses over UDP.

use std::io;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Duration;

use super::message::{DnsMessage, DnsResponseCode};
use super::record::{DnsRecordType, DnsRecordData};

/// DNS client for sending queries.
///
/// # Example
/// ```no_run
/// use edgerun_dns::DnsClient;
/// use edgerun_dns::record::DnsRecordType;
///
/// let mut client = DnsClient::new("8.8.8.8:53").unwrap();
/// match client.query_a("www.example.com") {
///     Ok(ips) => println!("IPs: {:?}", ips),
///     Err(e) => eprintln!("DNS query failed: {}", e),
/// }
/// ```
pub struct DnsClient {
    socket: UdpSocket,
    server: SocketAddr,
    next_id: u16,
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

        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(Duration::from_secs(5)))?;

        Ok(Self {
            socket,
            server,
            next_id: 0x1000,
        })
    }

    /// Query for A records (IPv4 addresses).
    pub fn query_a(&mut self, name: &str) -> Result<Vec<Ipv4Addr>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::A)?;

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
    pub fn query_aaaa(&mut self, name: &str) -> Result<Vec<std::net::Ipv6Addr>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::AAAA)?;

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
    pub fn query_cname(&mut self, name: &str) -> Result<Vec<String>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::CNAME)?;

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
    pub fn query_mx(&mut self, name: &str) -> Result<Vec<(u16, String)>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::MX)?;

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
    pub fn query_txt(&mut self, name: &str) -> Result<Vec<String>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::TXT)?;

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
    pub fn query_ns(&mut self, name: &str) -> Result<Vec<String>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::NS)?;

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
    pub fn query_ptr(&mut self, name: &str) -> Result<Vec<String>, io::Error> {
        let msg = self.send_query(name, DnsRecordType::PTR)?;

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

    /// Send a generic query and return the full response message.
    pub fn query(&mut self, name: &str, qtype: DnsRecordType) -> Result<DnsMessage, io::Error> {
        self.send_query(name, qtype)
    }

    // --- Internal ---

    fn send_query(&mut self, name: &str, qtype: DnsRecordType) -> Result<DnsMessage, io::Error> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let msg = DnsMessage::query(id, name.to_string(), qtype);
        let wire = msg.to_wire();

        self.socket.send_to(&wire, self.server)?;

        // Read response
        let mut buf = [0u8; 4096];
        let n = match self.socket.recv(&mut buf) {
            Ok(n) => n,
            Err(e) => return Err(e),
        };

        let response = DnsMessage::from_wire(&buf[..n])?;

        if response.header.id != id {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DNS response ID mismatch",
            ));
        }

        if response.header.response_code != DnsResponseCode::NoError {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "DNS error: {}",
                    response.header.response_code.as_str()
                ),
            ));
        }

        Ok(response)
    }

    /// Set the query timeout.
    pub fn set_timeout(&self, timeout: Duration) -> Result<(), io::Error> {
        self.socket.set_read_timeout(Some(timeout))
    }
}

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
        // Just verify the query message builds correctly
        let msg = DnsMessage::query(
            0x1234,
            "example.com".to_string(),
            DnsRecordType::A,
        );
        let wire = msg.to_wire();
        assert!(wire.len() > 12);
    }
}
