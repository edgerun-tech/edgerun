//! Runtime-owned HTTP client for node-managed transports.
//!
//! HTTP message syntax lives in `edgerun-protocols`. This module owns outbound
//! client transport because opening TCP/TLS connections is a node capability.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;
use core::pin::Pin;
use core::task::{Context, Poll};

pub use edgerun_protocols::http::{
    HeaderMap, HeaderName, HeaderValue, HttpRequest as Request, HttpResponse as Response, Method,
    Scheme, StatusCode, Uri,
};

use crate::rt::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpVersion {
    Http1,
}

#[derive(Debug)]
pub enum HttpClientError {
    InvalidUri(String),
    InvalidResponse(String),
    UnsupportedScheme(String),
    Io(rt::IoError),
    #[cfg(feature = "tls")]
    Tls(crate::tls::TlsError),
}

impl fmt::Display for HttpClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUri(error) => write!(f, "invalid URI: {error}"),
            Self::InvalidResponse(error) => write!(f, "invalid HTTP response: {error}"),
            Self::UnsupportedScheme(scheme) => write!(f, "unsupported HTTP scheme: {scheme}"),
            Self::Io(error) => write!(f, "HTTP transport I/O error: {error}"),
            #[cfg(feature = "tls")]
            Self::Tls(error) => write!(f, "HTTP TLS error: {error}"),
        }
    }
}

impl core::error::Error for HttpClientError {}

impl From<rt::IoError> for HttpClientError {
    fn from(error: rt::IoError) -> Self {
        Self::Io(error)
    }
}

#[cfg(feature = "tls")]
impl From<crate::tls::TlsError> for HttpClientError {
    fn from(error: crate::tls::TlsError) -> Self {
        Self::Tls(error)
    }
}

pub type Result<T> = core::result::Result<T, HttpClientError>;

#[derive(Clone, Debug)]
pub struct HttpClient {
    version: HttpVersion,
}

impl HttpClient {
    pub const fn new() -> Self {
        Self {
            version: HttpVersion::Http1,
        }
    }

    pub const fn version(mut self, version: HttpVersion) -> Self {
        self.version = version;
        self
    }

    pub const fn no_redirects(self) -> Self {
        self
    }

    pub async fn get(&self, uri: &str) -> Result<Response> {
        self.request(Method::GET, uri, None).await
    }

    pub async fn post(&self, uri: &str, body: impl Into<Vec<u8>>) -> Result<Response> {
        self.request(Method::POST, uri, Some(body.into())).await
    }

    pub async fn put(&self, uri: &str, body: impl Into<Vec<u8>>) -> Result<Response> {
        self.request(Method::PUT, uri, Some(body.into())).await
    }

    pub async fn delete(&self, uri: &str) -> Result<Response> {
        self.request(Method::DELETE, uri, None).await
    }

    pub async fn request(
        &self,
        method: Method,
        uri: &str,
        body: Option<Vec<u8>>,
    ) -> Result<Response> {
        let uri = Uri::parse(uri).map_err(HttpClientError::InvalidUri)?;
        let request = build_request(method, uri, body)?;
        self.execute(&request).await
    }

    pub async fn execute(&self, request: &Request) -> Result<Response> {
        match self.version {
            HttpVersion::Http1 => execute_http1(request).await,
        }
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

fn build_request(method: Method, uri: Uri, body: Option<Vec<u8>>) -> Result<Request> {
    let mut headers = HeaderMap::new();
    let Some(host) = uri.host() else {
        return Err(HttpClientError::InvalidUri("missing host".into()));
    };
    let _ = headers.insert("Host", &host_header_value(&uri, host));
    let _ = headers.insert("Connection", "close");
    let _ = headers.insert("User-Agent", "edgerun-node/0.1");
    if let Some(body) = body.as_ref() {
        let _ = headers.insert("Content-Length", &body.len().to_string());
    }
    Ok(Request::new(method, uri, headers, body))
}

fn host_header_value(uri: &Uri, host: &str) -> String {
    match uri.port() {
        Some(80) if !uri.is_https() => host.to_string(),
        Some(443) if uri.is_https() => host.to_string(),
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    }
}

async fn execute_http1(request: &Request) -> Result<Response> {
    let uri = request.uri();
    let Some(host) = uri.host() else {
        return Err(HttpClientError::InvalidUri("missing host".into()));
    };
    let port = uri.port().unwrap_or(if uri.is_https() { 443 } else { 80 });
    let target = format!("{host}:{port}");
    let tcp = rt::ConnectFuture::new(&target).await?;
    let mut stream = RuntimeHttpStream::connect(tcp, uri).await?;
    stream.write_all(&request.to_http_bytes()).await?;
    stream.flush().await?;

    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => break,
            Ok(read) => raw.extend_from_slice(&buf[..read]),
            Err(rt::IoError::UnexpectedEof) => break,
            Err(error) => return Err(error.into()),
        }
    }
    let raw = core::str::from_utf8(&raw)
        .map_err(|_| HttpClientError::InvalidResponse("response is not valid UTF-8".into()))?;
    Response::from_http(raw).map_err(|error| HttpClientError::InvalidResponse(error.to_string()))
}

enum RuntimeHttpStream {
    Plain(Arc<rt::AsyncTcpStream>),
    #[cfg(feature = "tls")]
    Tls(crate::tls::AsyncTlsStream<Arc<rt::AsyncTcpStream>>),
}

impl RuntimeHttpStream {
    async fn connect(tcp: Arc<rt::AsyncTcpStream>, uri: &Uri) -> Result<Self> {
        match uri.scheme() {
            Scheme::Http => Ok(Self::Plain(tcp)),
            Scheme::Https => {
                #[cfg(feature = "tls")]
                {
                    let host = uri
                        .host()
                        .ok_or_else(|| HttpClientError::InvalidUri("missing host".into()))?;
                    let tls = crate::tls::AsyncTlsStream::client(tcp, host, &[], None).await?;
                    Ok(Self::Tls(tls))
                }
                #[cfg(not(feature = "tls"))]
                {
                    Err(HttpClientError::UnsupportedScheme("https".into()))
                }
            }
            Scheme::Other(scheme) => Err(HttpClientError::UnsupportedScheme(scheme.clone())),
        }
    }
}

impl AsyncRead for RuntimeHttpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<rt::io::Result<usize>> {
        match &mut *self {
            Self::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for RuntimeHttpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<rt::io::Result<usize>> {
        match &mut *self {
            Self::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<rt::io::Result<()>> {
        match &mut *self {
            Self::Plain(stream) => Pin::new(stream).poll_flush(cx),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<rt::io::Result<()>> {
        match &mut *self {
            Self::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
