//! TLS integration for HTTP/2 servers.
//!
//! Provides a wrapper that combines `edgerun-tls` server handshake with
//! `edgerun-http` HTTP/2 connection management.
//!
//! # Example
//! ```no_run
//! use edgerun_http::tls::TlsHttp2Server;
//! use edgerun_http::http2::Connection;
//! use std::net::TcpStream;
//!
//! fn handle_client(stream: TcpStream) {
//!     let cert = edgerun_tls::certificate_gen::generate_self_signed(&["localhost"]);
//!     let mut server = TlsHttp2Server::accept(stream, &cert).unwrap();
//!     let conn = server.into_http2_connection();
//!     // Use conn as normal HTTP/2 connection...
//! }
//! ```

#[cfg(feature = "tls")]
use edgerun_tls::server::TlsServerStream;
#[cfg(feature = "tls")]
use edgerun_tls::certificate_gen::CertificateAndKey;
#[cfg(feature = "tls")]
use std::io::{self, Read, Write};

/// A TLS-wrapped HTTP/2 server connection.
///
/// Handles the TLS handshake using `edgerun-tls`, then provides access to
/// the underlying stream for HTTP/2 connection setup.
#[cfg(feature = "tls")]
pub struct TlsHttp2Server {
    /// The TLS stream after successful handshake
    tls_stream: TlsServerStream,
}

#[cfg(feature = "tls")]
impl TlsHttp2Server {
    /// Accept a TLS connection and perform the TLS handshake.
    ///
    /// After this returns successfully, the TLS handshake is complete
    /// and the connection is ready for HTTP/2 framing.
    pub fn accept(stream: std::net::TcpStream, cert: &CertificateAndKey) -> io::Result<Self> {
        let tls_stream = TlsServerStream::accept(stream, cert)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(Self { tls_stream })
    }

    /// Check if the TLS handshake is complete
    pub fn is_handshake_complete(&self) -> bool {
        self.tls_stream.is_handshake_complete()
    }

    /// Convert into an HTTP/2 connection.
    ///
    /// This consumes the TLS server and returns an HTTP/2 `Connection`
    /// that reads/writes over the encrypted TLS stream.
    pub fn into_http2_connection(self) -> Connection<TlsServerStream> {
        Connection::server(self.tls_stream)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
            .unwrap()
    }

    /// Get access to the underlying TLS stream for manual HTTP/2 handling.
    pub fn into_inner(self) -> TlsServerStream {
        self.tls_stream
    }
}

#[cfg(feature = "tls")]
impl Read for TlsHttp2Server {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.tls_stream.read(buf)
    }
}

#[cfg(feature = "tls")]
impl Write for TlsHttp2Server {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tls_stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tls_stream.flush()
    }
}

use crate::http2::Connection;

#[cfg(test)]
mod tests {
    #[cfg(feature = "tls")]
    mod tls_integration {
        use crate::tls::TlsHttp2Server;
        use edgerun_tls::certificate_gen::generate_self_signed;

        #[test]
        fn test_tls_http2_types_compile() {
            // Just verify the types exist and compile correctly
            let cert = generate_self_signed(&["127.0.0.1", "localhost"]);
            assert!(!cert.cert_der.is_empty());
            
            // Verify TlsHttp2Server type exists
            fn _assert_tls_server_exists() {
                let _ : fn(std::net::TcpStream, &edgerun_tls::certificate_gen::CertificateAndKey) -> std::io::Result<TlsHttp2Server> = 
                    TlsHttp2Server::accept;
            }
        }
    }
}
