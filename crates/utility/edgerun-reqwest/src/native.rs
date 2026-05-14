//! Reqwest-shaped native client backed by Edgerun HTTP transports.

#![allow(non_camel_case_types)]

use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

pub use edgerun_bytes::Bytes;
use edgerun_futures::Stream;
use edgerun_futures::StreamExt;
use edgerun_futures::future;
use edgerun_futures::stream;
pub use edgerun_http::header;
pub use edgerun_http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};

type BoxBodyStream = Pin<
    Box<
        dyn Stream<Item = Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>>
            + Send
            + 'static,
    >,
>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Url {
    inner: edgerun_url::Url,
    raw: String,
}

impl Url {
    pub fn parse(input: &str) -> Result<Self, edgerun_url::ParseError> {
        let inner = edgerun_url::Url::parse(input)?;
        let mut raw = inner.to_string();
        if inner.path().is_empty() {
            if let Some(pos) = raw.find('?').or_else(|| raw.find('#')) {
                raw.insert(pos, '/');
            } else {
                raw.push('/');
            }
        }
        Ok(Self { raw, inner })
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    pub fn scheme(&self) -> &str {
        self.inner.scheme()
    }

    pub fn host_str(&self) -> Option<&str> {
        Some(self.inner.host())
    }

    pub fn port(&self) -> Option<u16> {
        self.inner.port()
    }

    pub fn path(&self) -> &str {
        self.inner.path()
    }

    pub fn query(&self) -> Option<&str> {
        self.inner.query()
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Version {
    HTTP_09,
    HTTP_10,
    HTTP_11,
    HTTP_2,
    HTTP_3,
}

pub mod tls {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Version {
        TLS_1_2,
        TLS_1_3,
    }
}

pub mod redirect {
    #[derive(Clone, Debug)]
    pub struct Policy {
        pub(crate) max_redirects: Option<usize>,
    }

    impl Policy {
        pub fn none() -> Self {
            Self {
                max_redirects: Some(0),
            }
        }

        pub fn limited(max: usize) -> Self {
            Self {
                max_redirects: Some(max),
            }
        }
    }
}

pub mod cookie {
    use super::*;

    pub trait CookieStore: Send + Sync {
        fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url);

        fn cookies(&self, url: &Url) -> Option<HeaderValue>;
    }

    #[derive(Debug, Default)]
    pub struct Jar {
        cookies: Mutex<Vec<StoredCookie>>,
    }

    #[derive(Debug, Clone)]
    struct StoredCookie {
        host: String,
        name: String,
        value: String,
        secure: bool,
    }

    impl CookieStore for Jar {
        fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
            let Some(host) = url.host_str() else {
                return;
            };
            let secure_url = url.scheme() == "https";
            let mut cookies = self
                .cookies
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for header in cookie_headers {
                let Ok(header) = header.to_str() else {
                    continue;
                };
                let Some((name, rest)) = header.split_once('=') else {
                    continue;
                };
                let name = name.trim();
                if name.is_empty() {
                    continue;
                }
                let value = rest.split(';').next().unwrap_or_default().trim();
                let secure = header
                    .split(';')
                    .skip(1)
                    .any(|part| part.trim().eq_ignore_ascii_case("secure"));
                if secure && !secure_url {
                    continue;
                }
                cookies.retain(|cookie| !(cookie.host == host && cookie.name == name));
                cookies.push(StoredCookie {
                    host: host.to_string(),
                    name: name.to_string(),
                    value: value.to_string(),
                    secure,
                });
            }
        }

        fn cookies(&self, url: &Url) -> Option<HeaderValue> {
            let host = url.host_str()?;
            let secure_url = url.scheme() == "https";
            let cookies = self
                .cookies
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let header = cookies
                .iter()
                .filter(|cookie| cookie.host == host && (!cookie.secure || secure_url))
                .map(|cookie| format!("{}={}", cookie.name, cookie.value))
                .collect::<Vec<_>>()
                .join("; ");
            if header.is_empty() {
                None
            } else {
                HeaderValue::from_str(&header).ok()
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Proxy {
    url: String,
}

impl Proxy {
    pub fn https(url: &str) -> Result<Self, Error> {
        let parsed = Url::parse(url).map_err(|error| Error::url_error(error.to_string()))?;
        match parsed.scheme() {
            "http" | "https" => Ok(Self {
                url: parsed.to_string(),
            }),
            scheme => Err(Error::url_error(format!(
                "unsupported proxy scheme: {scheme}"
            ))),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

#[derive(Clone, Debug)]
pub struct Certificate {
    der: Arc<Vec<u8>>,
}

impl Certificate {
    pub fn from_der(der: &[u8]) -> Result<Self, Error> {
        if der.is_empty() {
            return Err(Error::builder("empty certificate DER"));
        }
        Ok(Self {
            der: Arc::new(der.to_vec()),
        })
    }

    pub fn as_der(&self) -> &[u8] {
        &self.der
    }
}

#[derive(Default)]
pub struct Body {
    kind: BodyKind,
}

enum BodyKind {
    Bytes(Vec<u8>),
    Stream(BoxBodyStream),
}

impl Default for BodyKind {
    fn default() -> Self {
        Self::Bytes(Vec::new())
    }
}

impl Body {
    pub fn from(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            kind: BodyKind::Bytes(bytes.into()),
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn wrap_stream<S, B, E>(stream: S) -> Self
    where
        S: Stream<Item = Result<B, E>> + Send + 'static,
        B: AsRef<[u8]> + Send + 'static,
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            kind: BodyKind::Stream(Box::pin(stream.map(|chunk| {
                chunk
                    .map(|bytes| bytes.as_ref().to_vec())
                    .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)
            }))),
        }
    }

    async fn into_bytes(self) -> Result<Vec<u8>, Error> {
        match self.kind {
            BodyKind::Bytes(bytes) => Ok(bytes),
            BodyKind::Stream(mut stream) => {
                let mut out = Vec::new();
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk.map_err(|error| Error::http(error.to_string()))?;
                    out.extend_from_slice(&chunk);
                }
                Ok(out)
            }
        }
    }
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            BodyKind::Bytes(bytes) => f.debug_tuple("Body::Bytes").field(&bytes.len()).finish(),
            BodyKind::Stream(_) => f.write_str("Body::Stream(..)"),
        }
    }
}

impl From<Vec<u8>> for Body {
    fn from(value: Vec<u8>) -> Self {
        Self::from(value)
    }
}

impl From<Bytes> for Body {
    fn from(value: Bytes) -> Self {
        Self::from(value.to_vec())
    }
}

impl From<&[u8]> for Body {
    fn from(value: &[u8]) -> Self {
        Self::from(value.to_vec())
    }
}

impl From<String> for Body {
    fn from(value: String) -> Self {
        Self::from(value.into_bytes())
    }
}

impl From<&str> for Body {
    fn from(value: &str) -> Self {
        Self::from(value.as_bytes().to_vec())
    }
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    status: Option<StatusCode>,
    url: Option<Url>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ErrorKind {
    Builder,
    Decode,
    Http,
    Redirect,
    Status,
    Timeout,
    Url,
}

impl Error {
    fn builder(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Builder,
            message: message.into(),
            status: None,
            url: None,
        }
    }

    fn decode(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Decode,
            message: message.into(),
            status: None,
            url: None,
        }
    }

    fn http(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Http,
            message: message.into(),
            status: None,
            url: None,
        }
    }

    fn status_with_url(status: StatusCode, url: Option<Url>) -> Self {
        let class = if status.is_client_error() {
            "client"
        } else if status.is_server_error() {
            "server"
        } else {
            "unknown"
        };
        let message = if let Some(url) = url.as_ref() {
            format!("HTTP status {class} error ({status}) for url ({url})")
        } else {
            format!("HTTP status {class} error ({status})")
        };
        Self {
            kind: ErrorKind::Status,
            message,
            status: Some(status),
            url,
        }
    }

    fn url_error(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Url,
            message: message.into(),
            status: None,
            url: None,
        }
    }

    pub fn is_builder(&self) -> bool {
        self.kind == ErrorKind::Builder
    }

    pub fn is_decode(&self) -> bool {
        self.kind == ErrorKind::Decode
    }

    pub fn is_redirect(&self) -> bool {
        self.kind == ErrorKind::Redirect
    }

    pub fn is_status(&self) -> bool {
        self.kind == ErrorKind::Status
    }

    pub fn is_timeout(&self) -> bool {
        self.kind == ErrorKind::Timeout
    }

    pub fn status(&self) -> Option<StatusCode> {
        self.status
    }

    pub fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

#[derive(Clone)]
pub struct Client {
    inner: edgerun_http_client::http::HttpClient,
    default_headers: HeaderMap,
    cookie_store: Option<Arc<dyn cookie::CookieStore>>,
    proxy: Option<Proxy>,
}

impl Client {
    pub fn new() -> Self {
        Self::builder()
            .build()
            .expect("default Edgerun HTTP client")
    }

    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::GET, url)
    }

    pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::POST, url)
    }

    pub fn put<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::PUT, url)
    }

    pub fn patch<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::PATCH, url)
    }

    pub fn delete<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::DELETE, url)
    }

    pub fn head<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::HEAD, url)
    }

    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
        RequestBuilder {
            client: self.clone(),
            method,
            url: url.into_url(),
            headers: HeaderMap::new(),
            body: None,
            timeout: None,
        }
    }

    pub async fn execute(&self, request: Request) -> Result<Response, Error> {
        execute_edgerun(self, request, None).await
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("default_headers", &self.default_headers)
            .field("cookie_store", &self.cookie_store.is_some())
            .field("proxy", &self.proxy)
            .finish_non_exhaustive()
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct ClientBuilder {
    version: edgerun_http_client::http::HttpVersion,
    connect_timeout: Option<Duration>,
    timeout: Option<Duration>,
    redirects: Option<usize>,
    decompress: bool,
    tls12_first: bool,
    invalid_http3_certs: bool,
    roots_der: Vec<Vec<u8>>,
    default_headers: HeaderMap,
    cookie_store: Option<Arc<dyn cookie::CookieStore>>,
    proxy: Option<Proxy>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            version: edgerun_http_client::http::HttpVersion::Best,
            connect_timeout: None,
            timeout: None,
            redirects: None,
            decompress: true,
            tls12_first: false,
            invalid_http3_certs: false,
            roots_der: Vec::new(),
            default_headers: HeaderMap::new(),
            cookie_store: None,
            proxy: None,
        }
    }

    pub fn build(self) -> Result<Client, Error> {
        let mut inner = edgerun_http_client::http::HttpClient::new().version(self.version);
        if let Some(timeout) = self.connect_timeout {
            inner = inner.with_connect_timeout(timeout);
        }
        if let Some(timeout) = self.timeout {
            inner = inner.with_read_timeout(timeout);
        }
        if let Some(max) = self.redirects {
            let max = u8::try_from(max).map_err(|_| Error::builder("redirect limit exceeds u8"))?;
            inner = inner.with_max_redirects(max);
        }
        if !self.decompress {
            inner = inner.no_decompress();
        }
        if self.tls12_first {
            inner = inner.with_tls12_first(true);
        }
        if self.invalid_http3_certs {
            inner = inner.danger_accept_invalid_http3_certs(true);
        }
        if !self.roots_der.is_empty() {
            inner = inner.with_http3_trust_roots_der(self.roots_der);
        }
        Ok(Client {
            inner,
            default_headers: self.default_headers,
            cookie_store: self.cookie_store,
            proxy: self.proxy,
        })
    }

    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn redirect(mut self, policy: redirect::Policy) -> Self {
        self.redirects = policy.max_redirects;
        self
    }

    pub fn no_redirect(mut self) -> Self {
        self.redirects = Some(0);
        self
    }

    pub fn no_gzip(mut self) -> Self {
        self.decompress = false;
        self
    }

    pub fn no_brotli(mut self) -> Self {
        self.decompress = false;
        self
    }

    pub fn no_deflate(mut self) -> Self {
        self.decompress = false;
        self
    }

    pub fn no_proxy(mut self) -> Self {
        self.proxy = None;
        self
    }

    pub fn use_rustls_tls(self) -> Self {
        self
    }

    pub fn min_tls_version(mut self, version: tls::Version) -> Self {
        self.tls12_first = matches!(version, tls::Version::TLS_1_2);
        self
    }

    pub fn http1_only(mut self) -> Self {
        self.version = edgerun_http_client::http::HttpVersion::Http1;
        self
    }

    pub fn http2_prior_knowledge(mut self) -> Self {
        self.version = edgerun_http_client::http::HttpVersion::Http2;
        self
    }

    pub fn http3_prior_knowledge(mut self) -> Self {
        self.version = edgerun_http_client::http::HttpVersion::Http3;
        self
    }

    pub fn danger_accept_invalid_certs(mut self, enabled: bool) -> Self {
        self.invalid_http3_certs = enabled;
        self
    }

    pub fn add_root_certificate(mut self, certificate: Certificate) -> Self {
        self.roots_der.push(certificate.as_der().to_vec());
        self
    }

    pub fn default_headers(mut self, headers: HeaderMap) -> Self {
        self.default_headers = headers;
        self
    }

    pub fn proxy(mut self, proxy: Proxy) -> Self {
        self.proxy = Some(proxy);
        self
    }

    pub fn cookie_provider<C>(mut self, provider: Arc<C>) -> Self
    where
        C: cookie::CookieStore + 'static,
    {
        let provider: Arc<dyn cookie::CookieStore> = provider;
        self.cookie_store = Some(provider);
        self
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RequestBuilder {
    client: Client,
    method: Method,
    url: Result<Url, Error>,
    headers: HeaderMap,
    body: Option<Body>,
    timeout: Option<Duration>,
}

impl fmt::Debug for RequestBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestBuilder")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl RequestBuilder {
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<edgerun_http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<edgerun_http::Error>,
    {
        let name = match HeaderName::try_from(key) {
            Ok(name) => name,
            Err(error) => {
                let error: edgerun_http::Error = error.into();
                self.url = Err(Error::builder(error.to_string()));
                return self;
            }
        };
        let value = match HeaderValue::try_from(value) {
            Ok(value) => value,
            Err(error) => {
                let error: edgerun_http::Error = error.into();
                self.url = Err(Error::builder(error.to_string()));
                return self;
            }
        };
        self.headers.insert(name, value);
        self
    }

    pub fn bearer_auth<T: fmt::Display>(self, token: T) -> Self {
        self.header(header::AUTHORIZATION, format!("Bearer {token}"))
    }

    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn body<B: Into<Body>>(mut self, body: B) -> Self {
        self.body = Some(body.into());
        self
    }

    #[cfg(feature = "json")]
    pub fn json<T: edgerun_json::ToJson + ?Sized>(self, value: &T) -> Self {
        self.json_edgerun(value)
    }

    pub fn json_edgerun<T: edgerun_json::ToJson + ?Sized>(self, value: &T) -> Self {
        match edgerun_json::to_json_vec(value) {
            Ok(body) => self
                .header(header::CONTENT_TYPE, "application/json")
                .body(body),
            Err(error) => RequestBuilder {
                url: Err(Error::builder(error.to_string())),
                ..self
            },
        }
    }

    pub fn build(self) -> Result<Request, Error> {
        let url = self.url?;
        let mut headers = self.client.default_headers.clone();
        headers.extend(self.headers);
        Ok(Request {
            method: self.method,
            url,
            headers,
            body: self.body.unwrap_or_default(),
            timeout: self.timeout,
        })
    }

    pub async fn send(self) -> Result<Response, Error> {
        let client = self.client.clone();
        let request = self.build()?;
        let timeout = request.timeout;
        execute_edgerun(&client, request, timeout).await
    }
}

pub struct Request {
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: Body,
    timeout: Option<Duration>,
}

pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
    version: Version,
    url: Option<Url>,
}

impl Response {
    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub async fn bytes(self) -> Result<Bytes, Error> {
        Ok(self.body)
    }

    pub fn bytes_stream(self) -> impl Stream<Item = Result<Bytes, Error>> + Send + 'static {
        stream::once(future::ready(Ok(self.body)))
    }

    pub async fn text(self) -> Result<String, Error> {
        String::from_utf8(self.body.to_vec()).map_err(|error| Error::decode(error.to_string()))
    }

    #[cfg(feature = "json")]
    pub async fn json<T: edgerun_json::serde_bridge::DeserializeOwned>(self) -> Result<T, Error> {
        edgerun_json::from_serde_slice(&self.body).map_err(|error| Error::decode(error.to_string()))
    }

    pub async fn json_edgerun<T: edgerun_json::FromJson>(self) -> Result<T, Error> {
        edgerun_json::from_json_slice(&self.body).map_err(|error| Error::decode(error.to_string()))
    }

    pub fn error_for_status(self) -> Result<Self, Error> {
        if self.status.is_client_error() || self.status.is_server_error() {
            Err(Error::status_with_url(self.status, self.url.clone()))
        } else {
            Ok(self)
        }
    }

    pub fn error_for_status_ref(&self) -> Result<&Self, Error> {
        if self.status.is_client_error() || self.status.is_server_error() {
            Err(Error::status_with_url(self.status, self.url.clone()))
        } else {
            Ok(self)
        }
    }
}

pub trait IntoResponseBody {
    fn into_response_body(self) -> Vec<u8>;
}

impl IntoResponseBody for Vec<u8> {
    fn into_response_body(self) -> Vec<u8> {
        self
    }
}

impl IntoResponseBody for Bytes {
    fn into_response_body(self) -> Vec<u8> {
        self.to_vec()
    }
}

impl IntoResponseBody for String {
    fn into_response_body(self) -> Vec<u8> {
        self.into_bytes()
    }
}

impl IntoResponseBody for &str {
    fn into_response_body(self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl IntoResponseBody for &[u8] {
    fn into_response_body(self) -> Vec<u8> {
        self.to_vec()
    }
}

impl<T> From<edgerun_http::Response<T>> for Response
where
    T: IntoResponseBody,
{
    fn from(value: edgerun_http::Response<T>) -> Self {
        let status = value.status();
        let headers = value.headers().clone();
        let url = value.extensions().get::<Url>().cloned();
        let body = Bytes::from(value.into_body().into_response_body());
        Self {
            status,
            headers,
            body,
            version: Version::HTTP_11,
            url,
        }
    }
}

pub trait ResponseBuilderExt {
    fn url(self, url: Url) -> Self;
}

impl ResponseBuilderExt for edgerun_http::response::Builder {
    fn url(self, url: Url) -> Self {
        self.extension(url)
    }
}

pub trait IntoUrl {
    fn into_url(self) -> Result<Url, Error>;
    fn as_str(&self) -> &str;
}

impl IntoUrl for Url {
    fn into_url(self) -> Result<Url, Error> {
        Ok(self)
    }

    fn as_str(&self) -> &str {
        Url::as_str(self)
    }
}

impl IntoUrl for &str {
    fn into_url(self) -> Result<Url, Error> {
        Url::parse(self).map_err(|error| Error::url_error(error.to_string()))
    }

    fn as_str(&self) -> &str {
        self
    }
}

impl IntoUrl for String {
    fn into_url(self) -> Result<Url, Error> {
        self.as_str().into_url()
    }

    fn as_str(&self) -> &str {
        self.as_str()
    }
}

impl IntoUrl for &String {
    fn into_url(self) -> Result<Url, Error> {
        self.as_str().into_url()
    }

    fn as_str(&self) -> &str {
        String::as_str(self)
    }
}

fn convert_method(method: &Method) -> edgerun_http_client::http::Method {
    match method.as_str() {
        "GET" => edgerun_http_client::http::Method::GET,
        "POST" => edgerun_http_client::http::Method::POST,
        "PUT" => edgerun_http_client::http::Method::PUT,
        "DELETE" => edgerun_http_client::http::Method::DELETE,
        "PATCH" => edgerun_http_client::http::Method::PATCH,
        "HEAD" => edgerun_http_client::http::Method::HEAD,
        "OPTIONS" => edgerun_http_client::http::Method::OPTIONS,
        "CONNECT" => edgerun_http_client::http::Method::CONNECT,
        "TRACE" => edgerun_http_client::http::Method::TRACE,
        other => edgerun_http_client::http::Method::Extension(other.to_string()),
    }
}

fn convert_status(status: edgerun_http_client::http::StatusCode) -> Result<StatusCode, Error> {
    StatusCode::from_u16(status.as_u16()).map_err(|error| Error::http(error.to_string()))
}

fn convert_headers(headers: &HeaderMap) -> Result<edgerun_http_client::http::HeaderMap, Error> {
    let mut out = edgerun_http_client::http::HeaderMap::new();
    for (name, value) in headers {
        let value = value
            .to_str()
            .map_err(|error| Error::builder(error.to_string()))?;
        let _ = out.insert(name.as_str(), value);
    }
    Ok(out)
}

fn convert_response_headers(
    headers: &edgerun_http_client::http::HeaderMap,
) -> Result<HeaderMap, Error> {
    let mut out = HeaderMap::new();
    for (name, value) in headers.iter() {
        let name = HeaderName::from_bytes(name.as_str().as_bytes())
            .map_err(|error| Error::http(error.to_string()))?;
        let value = HeaderValue::from_str(value.as_str())
            .map_err(|error| Error::http(error.to_string()))?;
        out.insert(name, value);
    }
    Ok(out)
}

async fn execute_edgerun(
    client: &Client,
    request: Request,
    timeout: Option<Duration>,
) -> Result<Response, Error> {
    let response_url = request.url.clone();
    let body = request.body.into_bytes().await?;
    let body = if body.is_empty() { None } else { Some(body) };
    let mut headers = request.headers;
    if let Some(store) = client.cookie_store.as_ref()
        && let Some(cookies) = store.cookies(&request.url)
    {
        headers.insert(header::COOKIE, cookies);
    }

    if let Some(proxy) = client.proxy.as_ref()
        && request.url.scheme() == "https"
    {
        return execute_https_via_connect_proxy(
            proxy,
            &request.method,
            &request.url,
            headers,
            body,
            response_url,
        )
        .await;
    }

    let uri = request.url.to_string();
    let headers = convert_headers(&headers)?;
    let mut builder = edgerun_http_client::http::Request::builder()
        .method(convert_method(&request.method))
        .uri(&uri)
        .with_headers(headers);
    if let Some(body) = body {
        builder = builder.body(body);
    }
    let request = builder
        .build()
        .map_err(|error| Error::http(error.to_string()))?;
    let mut inner = client.inner.clone();
    if let Some(timeout) = timeout {
        inner = inner.with_read_timeout(timeout);
    }
    let response = inner
        .execute(&request)
        .await
        .map_err(|error| Error::http(error.to_string()))?;
    Ok(Response {
        status: convert_status(response.status())?,
        headers: convert_response_headers(response.headers())?,
        body: Bytes::from(response.body().to_vec()),
        version: Version::HTTP_11,
        url: Some(response_url),
    })
}

async fn execute_https_via_connect_proxy(
    proxy: &Proxy,
    method: &Method,
    url: &Url,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    response_url: Url,
) -> Result<Response, Error> {
    use edgerun_http_client::rt::AsyncRead;
    use edgerun_http_client::rt::AsyncReadExt;
    use edgerun_http_client::rt::AsyncWrite;
    use edgerun_http_client::rt::AsyncWriteExt;
    use edgerun_http_client::rt::ConnectFuture;
    use edgerun_http_client::tls::AsyncTlsStream;

    let proxy_url = Url::parse(proxy.url()).map_err(|error| Error::url_error(error.to_string()))?;
    if proxy_url.scheme() != "http" {
        return Err(Error::builder(format!(
            "unsupported HTTPS proxy transport scheme: {}",
            proxy_url.scheme()
        )));
    }
    let proxy_host = proxy_url
        .host_str()
        .ok_or_else(|| Error::url_error("proxy URL missing host"))?;
    let proxy_port = proxy_url.port().unwrap_or(80);
    let target_host = url
        .host_str()
        .ok_or_else(|| Error::url_error("target URL missing host"))?;
    let target_port = url.port().unwrap_or(443);
    let target_authority = format!("{target_host}:{target_port}");

    let mut stream = ConnectFuture::new(format!("{proxy_host}:{proxy_port}"))
        .await
        .map_err(|error| Error::http(error.to_string()))?;
    let connect_request = format!(
        "CONNECT {target_authority} HTTP/1.1\r\nHost: {target_authority}\r\nProxy-Connection: Keep-Alive\r\n\r\n"
    );
    AsyncWriteExt::write_all(&mut stream, connect_request.as_bytes())
        .await
        .map_err(|error| Error::http(error.to_string()))?;
    AsyncWriteExt::flush(&mut stream)
        .await
        .map_err(|error| Error::http(error.to_string()))?;
    let connect_response = read_http_message_async(&mut stream)
        .await
        .map_err(Error::http)?;
    let (connect_status, _, _) = parse_http_response(&connect_response)?;
    if !(200..300).contains(&connect_status) {
        return Err(Error::http(format!(
            "proxy CONNECT failed with status {connect_status}"
        )));
    }

    let mut tls = AsyncTlsStream::client(stream, target_host, &[], None)
        .await
        .map_err(|error| Error::http(error.to_string()))?;
    let body = body.unwrap_or_default();
    let request_target = request_target(url);
    let host_header = if url.port().is_some() {
        target_authority
    } else {
        target_host.to_string()
    };
    let mut request = format!(
        "{} {request_target} HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\nContent-Length: {}\r\n",
        method.as_str(),
        body.len()
    );
    for (name, value) in headers.iter() {
        if name.as_str().eq_ignore_ascii_case("host")
            || name.as_str().eq_ignore_ascii_case("content-length")
            || name.as_str().eq_ignore_ascii_case("connection")
        {
            continue;
        }
        let value = value
            .to_str()
            .map_err(|error| Error::builder(error.to_string()))?;
        request.push_str(name.as_str());
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");
    AsyncWriteExt::write_all(&mut tls, request.as_bytes())
        .await
        .map_err(|error| Error::http(error.to_string()))?;
    if !body.is_empty() {
        AsyncWriteExt::write_all(&mut tls, &body)
            .await
            .map_err(|error| Error::http(error.to_string()))?;
    }
    AsyncWriteExt::flush(&mut tls)
        .await
        .map_err(|error| Error::http(error.to_string()))?;

    let response = read_http_message_async(&mut tls)
        .await
        .map_err(Error::http)?;
    let (status, headers, body) = parse_http_response(&response)?;
    Ok(Response {
        status: StatusCode::from_u16(status).map_err(|error| Error::http(error.to_string()))?,
        headers,
        body: Bytes::from(body),
        version: Version::HTTP_11,
        url: Some(response_url),
    })
}

fn request_target(url: &Url) -> String {
    let mut target = if url.path().is_empty() {
        "/".to_string()
    } else {
        url.path().to_string()
    };
    if let Some(query) = url.query() {
        target.push('?');
        target.push_str(query);
    }
    target
}

async fn read_http_message_async<S>(stream: &mut S) -> Result<Vec<u8>, String>
where
    S: edgerun_http_client::rt::AsyncRead + Unpin,
{
    let mut buffer = Vec::new();
    let mut chunk = [0; 1024];
    loop {
        let bytes_read = edgerun_http_client::rt::AsyncReadExt::read(stream, &mut chunk)
            .await
            .map_err(|error| error.to_string())?;
        if bytes_read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..bytes_read]);
        if http_message_complete(&buffer) {
            break;
        }
    }
    Ok(buffer)
}

fn http_message_complete(buffer: &[u8]) -> bool {
    if let Some(header_end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
        let body_start = header_end + 4;
        let headers = String::from_utf8_lossy(&buffer[..body_start]);
        let content_length = headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find_map(|(name, value)| {
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        return buffer.len() >= body_start + content_length;
    }
    false
}

fn parse_http_response(buffer: &[u8]) -> Result<(u16, HeaderMap, Vec<u8>), Error> {
    let header_end = buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| Error::http("HTTP response missing header terminator"))?;
    let body_start = header_end + 4;
    let header_text = String::from_utf8_lossy(&buffer[..header_end]);
    let mut lines = header_text.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| Error::http("HTTP response missing status line"))?;
    let mut status_parts = status_line.split_whitespace();
    let _version = status_parts
        .next()
        .ok_or_else(|| Error::http("HTTP response missing version"))?;
    let status = status_parts
        .next()
        .ok_or_else(|| Error::http("HTTP response missing status code"))?
        .parse::<u16>()
        .map_err(|error| Error::http(error.to_string()))?;
    let mut headers = HeaderMap::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = HeaderName::from_bytes(name.trim().as_bytes())
            .map_err(|error| Error::http(error.to_string()))?;
        let value =
            HeaderValue::from_str(value.trim()).map_err(|error| Error::http(error.to_string()))?;
        headers.insert(name, value);
    }
    Ok((status, headers, buffer[body_start..].to_vec()))
}
