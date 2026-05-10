#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::fmt;
#[cfg(feature = "std")]
use edgerun_node::rt::{AsyncRead, AsyncWrite, IoError};
#[cfg(feature = "std")]
pub use edgerun_node::tls::{AsyncTlsServerStream, AsyncTlsStream};
use edgerun_protocols::tls::certificate_gen::CertificateAndKey;
#[cfg(feature = "std")]
use std::io::{self, Read, Write};
#[cfg(feature = "std")]
use std::pin::Pin;
#[cfg(feature = "std")]
use std::task::{Context, Poll};

pub mod version {
    #[derive(Debug)]
    pub struct SupportedProtocolVersion;

    pub static TLS13: SupportedProtocolVersion = SupportedProtocolVersion;
}

pub mod pki_types {
    use alloc::vec::Vec;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CertificateDer<'a>(pub Vec<u8>, core::marker::PhantomData<&'a ()>);

    impl<'a> CertificateDer<'a> {
        pub fn into_owned(self) -> CertificateDer<'static> {
            CertificateDer(self.0, core::marker::PhantomData)
        }
    }

    impl From<Vec<u8>> for CertificateDer<'static> {
        fn from(value: Vec<u8>) -> Self {
            Self(value, core::marker::PhantomData)
        }
    }

    impl AsRef<[u8]> for CertificateDer<'_> {
        fn as_ref(&self) -> &[u8] {
            &self.0
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct PrivateKeyDer<'a>(pub Vec<u8>, core::marker::PhantomData<&'a ()>);

    impl<'a> PrivateKeyDer<'a> {
        pub fn into_owned(self) -> PrivateKeyDer<'static> {
            PrivateKeyDer(self.0, core::marker::PhantomData)
        }
    }

    impl From<Vec<u8>> for PrivateKeyDer<'static> {
        fn from(value: Vec<u8>) -> Self {
            Self(value, core::marker::PhantomData)
        }
    }

    impl AsRef<[u8]> for PrivateKeyDer<'_> {
        fn as_ref(&self) -> &[u8] {
            &self.0
        }
    }
}

pub use pki_types::{CertificateDer, PrivateKeyDer};

#[derive(Debug)]
pub struct Error(String);

impl Error {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[derive(Clone)]
pub struct ServerConfig {
    certificate: CertificateAndKey,
}

#[derive(Clone, Default)]
pub struct ClientConfig;

pub struct ConfigBuilder;
pub struct ClientConfigBuilder;

impl ServerConfig {
    pub fn builder_with_protocol_versions(
        _versions: &[&'static version::SupportedProtocolVersion],
    ) -> ConfigBuilder {
        ConfigBuilder
    }

    pub fn from_certificate(certificate: CertificateAndKey) -> Self {
        Self { certificate }
    }

    pub fn certificate(&self) -> &CertificateAndKey {
        &self.certificate
    }
}

impl ConfigBuilder {
    pub fn with_no_client_auth(self) -> Self {
        self
    }

    pub fn with_single_cert(
        self,
        cert_chain: Vec<CertificateDer<'static>>,
        private_key: PrivateKeyDer<'static>,
    ) -> Result<ServerConfig, Error> {
        let cert = cert_chain
            .first()
            .ok_or_else(|| Error::new("missing TLS certificate"))?;
        let certificate = CertificateAndKey::from_der_pair(cert.as_ref(), private_key.as_ref())
            .map_err(|error| Error::new(format!("invalid TLS material: {error}")))?;
        Ok(ServerConfig { certificate })
    }
}

impl ClientConfig {
    pub fn builder_with_protocol_versions(
        _versions: &[&'static version::SupportedProtocolVersion],
    ) -> ClientConfigBuilder {
        ClientConfigBuilder
    }
}

impl ClientConfigBuilder {
    pub fn with_root_certificates(self, _roots: RootCertStore) -> Self {
        self
    }

    pub fn with_no_client_auth(self) -> ClientConfig {
        ClientConfig
    }
}

#[derive(Clone, Default)]
pub struct RootCertStore {
    certs: Vec<CertificateDer<'static>>,
}

impl RootCertStore {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn add(&mut self, cert: CertificateDer<'static>) -> Result<(), Error> {
        self.certs.push(cert);
        Ok(())
    }
}

#[derive(Clone)]
pub struct ServerConnection {
    config: Arc<ServerConfig>,
}

impl ServerConnection {
    pub fn new(config: Arc<ServerConfig>) -> Result<Self, Error> {
        Ok(Self { config })
    }
}

#[derive(Clone)]
pub struct ClientConnection {
    config: Arc<ClientConfig>,
    server_name: String,
}

impl ClientConnection {
    pub fn new(config: Arc<ClientConfig>, server_name: impl Into<String>) -> Result<Self, Error> {
        Ok(Self {
            config,
            server_name: server_name.into(),
        })
    }
}

#[cfg(feature = "std")]
pub struct StreamOwned<C, T: Read + Write> {
    pub conn: C,
    sock: Option<T>,
    server_tls: Option<AsyncTlsServerStream<BlockingIo<T>>>,
    client_tls: Option<AsyncTlsStream<BlockingIo<T>>>,
}

#[cfg(feature = "std")]
impl<T: Read + Write + Unpin> StreamOwned<ServerConnection, T> {
    pub fn new(conn: ServerConnection, sock: T) -> Self {
        Self {
            conn,
            sock: Some(sock),
            server_tls: None,
            client_tls: None,
        }
    }

    pub fn get_ref(&self) -> &T {
        self.sock
            .as_ref()
            .or_else(|| self.server_tls.as_ref().map(|tls| tls.get_ref().get_ref()))
            .expect("TLS stream has an underlying transport")
    }

    pub fn get_mut(&mut self) -> &mut T {
        if let Some(sock) = self.sock.as_mut() {
            sock
        } else {
            self.server_tls
                .as_mut()
                .map(|tls| tls.get_mut().get_mut())
                .expect("TLS stream has an underlying transport")
        }
    }

    pub fn into_inner(mut self) -> T {
        if let Some(tls) = self.server_tls.take() {
            tls.into_inner().into_inner()
        } else {
            self.sock.expect("TLS stream has an underlying transport")
        }
    }

    pub fn handshake(&mut self) -> io::Result<()> {
        let _ = self.ensure_handshake_blocking()?;
        Ok(())
    }

    fn ensure_handshake(&mut self) -> io::Result<&mut AsyncTlsServerStream<BlockingIo<T>>> {
        if self.server_tls.is_none() {
            return self.ensure_handshake_blocking();
        }
        Ok(self.server_tls.as_mut().expect("TLS stream initialized"))
    }

    fn ensure_handshake_blocking(
        &mut self,
    ) -> io::Result<&mut AsyncTlsServerStream<BlockingIo<T>>> {
        if self.server_tls.is_none() {
            let sock = self
                .sock
                .take()
                .expect("TLS stream has an underlying transport");
            let stream = BlockingIo::new(sock);
            let cert = self.conn.config.certificate.clone();
            let tls = edgerun_node::rt::block_on(AsyncTlsServerStream::accept(stream, &cert))
                .map_err(|error| io::Error::other(error.to_string()))?;
            self.server_tls = Some(tls);
        }
        Ok(self.server_tls.as_mut().expect("TLS stream initialized"))
    }
}

#[cfg(feature = "std")]
impl<T: Read + Write + Unpin> Read for StreamOwned<ServerConnection, T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let tls = self.ensure_handshake()?;
        poll_once(|cx| tls.poll_read(cx, buf))
    }
}

#[cfg(feature = "std")]
impl<T: Read + Write + Unpin> StreamOwned<ClientConnection, T> {
    pub fn new(conn: ClientConnection, sock: T) -> Self {
        Self {
            conn,
            sock: Some(sock),
            server_tls: None,
            client_tls: None,
        }
    }

    pub fn get_ref(&self) -> &T {
        self.sock
            .as_ref()
            .or_else(|| self.client_tls.as_ref().map(|tls| tls.get_ref().get_ref()))
            .expect("TLS stream has an underlying transport")
    }

    pub fn get_mut(&mut self) -> &mut T {
        if let Some(sock) = self.sock.as_mut() {
            sock
        } else {
            self.client_tls
                .as_mut()
                .map(|tls| tls.get_mut().get_mut())
                .expect("TLS stream has an underlying transport")
        }
    }

    pub fn into_inner(mut self) -> T {
        if let Some(tls) = self.client_tls.take() {
            tls.into_inner().into_inner()
        } else {
            self.sock.expect("TLS stream has an underlying transport")
        }
    }

    pub fn handshake(&mut self) -> io::Result<()> {
        let _ = self.ensure_handshake_blocking()?;
        Ok(())
    }

    fn ensure_handshake(&mut self) -> io::Result<&mut AsyncTlsStream<BlockingIo<T>>> {
        if self.client_tls.is_none() {
            return self.ensure_handshake_blocking();
        }
        Ok(self.client_tls.as_mut().expect("TLS stream initialized"))
    }

    fn ensure_handshake_blocking(&mut self) -> io::Result<&mut AsyncTlsStream<BlockingIo<T>>> {
        if self.client_tls.is_none() {
            let sock = self
                .sock
                .take()
                .expect("TLS stream has an underlying transport");
            let stream = BlockingIo::new(sock);
            let _config = Arc::clone(&self.conn.config);
            let server_name = self.conn.server_name.clone();
            let tls = edgerun_node::rt::block_on(AsyncTlsStream::client(
                stream,
                &server_name,
                &[b"http/1.1"],
                None,
            ))
            .map_err(|error| io::Error::other(error.to_string()))?;
            self.client_tls = Some(tls);
        }
        Ok(self.client_tls.as_mut().expect("TLS stream initialized"))
    }
}

#[cfg(feature = "std")]
impl<T: Read + Write + Unpin> Read for StreamOwned<ClientConnection, T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let tls = self.ensure_handshake()?;
        poll_once(|cx| tls.poll_read(cx, buf))
    }
}

#[cfg(feature = "std")]
impl<T: Read + Write + Unpin> Write for StreamOwned<ClientConnection, T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let tls = self.ensure_handshake()?;
        poll_blocking(|cx| tls.poll_write(cx, buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        let tls = self.ensure_handshake()?;
        poll_blocking(|cx| tls.poll_flush(cx))
    }
}

#[cfg(feature = "std")]
impl<T: Read + Write + Unpin> Write for StreamOwned<ServerConnection, T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let tls = self.ensure_handshake()?;
        poll_blocking(|cx| tls.poll_write(cx, buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        let tls = self.ensure_handshake()?;
        poll_blocking(|cx| tls.poll_flush(cx))
    }
}

#[cfg(feature = "std")]
struct BlockingIo<T> {
    inner: T,
}

#[cfg(feature = "std")]
impl<T> BlockingIo<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }

    fn into_inner(self) -> T {
        self.inner
    }

    fn get_ref(&self) -> &T {
        &self.inner
    }

    fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

#[cfg(feature = "std")]
impl<T: Read + Unpin> AsyncRead for BlockingIo<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<edgerun_node::rt::io::Result<usize>> {
        match self.inner.read(buf) {
            Ok(value) => Poll::Ready(Ok(value)),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(error) => Poll::Ready(Err(io_error(error))),
        }
    }
}

#[cfg(feature = "std")]
impl<T: Write + Unpin> AsyncWrite for BlockingIo<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<edgerun_node::rt::io::Result<usize>> {
        match self.inner.write(buf) {
            Ok(value) => Poll::Ready(Ok(value)),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(error) => Poll::Ready(Err(io_error(error))),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<edgerun_node::rt::io::Result<()>> {
        match self.inner.flush() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(error) => Poll::Ready(Err(io_error(error))),
        }
    }
}

#[cfg(feature = "std")]
fn poll_blocking<T>(
    mut f: impl FnMut(&mut Context<'_>) -> Poll<edgerun_node::rt::io::Result<T>>,
) -> io::Result<T> {
    let waker = edgerun_node::rt::noop_waker();
    let mut cx = Context::from_waker(&waker);
    loop {
        match f(&mut cx) {
            Poll::Ready(Ok(value)) => return Ok(value),
            Poll::Ready(Err(error)) => return Err(io::Error::other(error.to_string())),
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

#[cfg(feature = "std")]
fn poll_once<T>(
    mut f: impl FnMut(&mut Context<'_>) -> Poll<edgerun_node::rt::io::Result<T>>,
) -> io::Result<T> {
    let waker = edgerun_node::rt::noop_waker();
    let mut cx = Context::from_waker(&waker);
    match f(&mut cx) {
        Poll::Ready(Ok(value)) => Ok(value),
        Poll::Ready(Err(error)) => Err(io::Error::other(error.to_string())),
        Poll::Pending => Err(io::Error::from(io::ErrorKind::WouldBlock)),
    }
}

#[cfg(feature = "std")]
fn io_error(error: io::Error) -> IoError {
    match error.kind() {
        io::ErrorKind::UnexpectedEof => IoError::UnexpectedEof,
        io::ErrorKind::WriteZero => IoError::WriteZero,
        io::ErrorKind::WouldBlock => IoError::Other("would block"),
        _ => IoError::Other("host I/O error"),
    }
}
