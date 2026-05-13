//! EdgeRun-owned HTTP compatibility types used by the Codex crates.

use std::any::{Any, TypeId};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::btree_map;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Index;
use std::str::FromStr;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl From<std::convert::Infallible> for Error {
    fn from(value: std::convert::Infallible) -> Self {
        match value {}
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
enum NameRepr {
    Static(&'static str),
    Owned(String),
}

#[derive(Clone)]
pub struct HeaderName(NameRepr);

impl HeaderName {
    pub const fn from_static(value: &'static str) -> Self {
        Self(NameRepr::Static(value))
    }

    pub fn from_bytes(value: &[u8]) -> Result<Self> {
        let value =
            std::str::from_utf8(value).map_err(|_| Error::new("invalid header name utf-8"))?;
        Self::try_from(value)
    }

    pub fn as_str(&self) -> &str {
        match &self.0 {
            NameRepr::Static(value) => value,
            NameRepr::Owned(value) => value.as_str(),
        }
    }
}

impl fmt::Debug for HeaderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("HeaderName").field(&self.as_str()).finish()
    }
}

impl fmt::Display for HeaderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq for HeaderName {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for HeaderName {}

impl PartialOrd for HeaderName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeaderName {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for HeaderName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl FromStr for HeaderName {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::try_from(value)
    }
}

impl TryFrom<&str> for HeaderName {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        if value.is_empty() {
            return Err(Error::new("empty header name"));
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
        {
            return Err(Error::new("invalid header name"));
        }
        Ok(Self(NameRepr::Owned(value.to_ascii_lowercase())))
    }
}

impl TryFrom<String> for HeaderName {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&String> for HeaderName {
    type Error = Error;

    fn try_from(value: &String) -> Result<Self> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&HeaderName> for HeaderName {
    type Error = Error;

    fn try_from(value: &HeaderName) -> Result<Self> {
        Ok(value.clone())
    }
}

pub trait IntoHeaderName {
    fn into_header_name(self) -> HeaderName;
}

impl IntoHeaderName for HeaderName {
    fn into_header_name(self) -> HeaderName {
        self
    }
}

impl IntoHeaderName for &HeaderName {
    fn into_header_name(self) -> HeaderName {
        self.clone()
    }
}

impl IntoHeaderName for &str {
    fn into_header_name(self) -> HeaderName {
        HeaderName::try_from(self).expect("invalid header name")
    }
}

impl IntoHeaderName for String {
    fn into_header_name(self) -> HeaderName {
        HeaderName::try_from(self).expect("invalid header name")
    }
}

impl IntoHeaderName for &String {
    fn into_header_name(self) -> HeaderName {
        HeaderName::try_from(self).expect("invalid header name")
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct HeaderValue(Vec<u8>);

impl HeaderValue {
    pub fn from_static(value: &'static str) -> Self {
        Self(value.as_bytes().to_vec())
    }

    pub fn from_str(value: &str) -> Result<Self> {
        if value.bytes().any(|byte| byte < 0x20 && byte != b'\t') {
            return Err(Error::new("invalid header value"));
        }
        Ok(Self(value.as_bytes().to_vec()))
    }

    pub fn from_bytes(value: &[u8]) -> Result<Self> {
        if value.iter().any(|byte| *byte < 0x20 && *byte != b'\t') {
            return Err(Error::new("invalid header value"));
        }
        Ok(Self(value.to_vec()))
    }

    pub fn to_str(&self) -> Result<&str> {
        std::str::from_utf8(&self.0).map_err(|_| Error::new("header value is not utf-8"))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for HeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_str() {
            Ok(value) => f.debug_tuple("HeaderValue").field(&value).finish(),
            Err(_) => f
                .debug_tuple("HeaderValue")
                .field(&format_args!("{} bytes", self.0.len()))
                .finish(),
        }
    }
}

impl TryFrom<&str> for HeaderValue {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        Self::from_str(value)
    }
}

impl TryFrom<String> for HeaderValue {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::from_str(&value)
    }
}

impl TryFrom<&String> for HeaderValue {
    type Error = Error;

    fn try_from(value: &String) -> Result<Self> {
        Self::from_str(value)
    }
}

impl TryFrom<u64> for HeaderValue {
    type Error = Error;

    fn try_from(value: u64) -> Result<Self> {
        Self::from_str(&value.to_string())
    }
}

impl TryFrom<&HeaderValue> for HeaderValue {
    type Error = Error;

    fn try_from(value: &HeaderValue) -> Result<Self> {
        Ok(value.clone())
    }
}

impl FromStr for HeaderValue {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::from_str(value)
    }
}

impl PartialEq<&str> for HeaderValue {
    fn eq(&self, other: &&str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<HeaderValue> for &str {
    fn eq(&self, other: &HeaderValue) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

#[derive(Clone, Default, Eq, PartialEq)]
pub struct HeaderMap {
    inner: BTreeMap<HeaderName, HeaderValue>,
}

impl HeaderMap {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    pub fn with_capacity(_capacity: usize) -> Self {
        Self::new()
    }

    pub fn insert<K: IntoHeaderName>(&mut self, key: K, value: HeaderValue) -> Option<HeaderValue> {
        self.inner.insert(key.into_header_name(), value)
    }

    pub fn append<K: IntoHeaderName>(&mut self, key: K, value: HeaderValue) -> bool {
        self.inner.insert(key.into_header_name(), value);
        true
    }

    pub fn get<K: IntoHeaderName>(&self, key: K) -> Option<&HeaderValue> {
        self.inner.get(&key.into_header_name())
    }

    pub fn contains_key<K: IntoHeaderName>(&self, key: K) -> bool {
        self.inner.contains_key(&key.into_header_name())
    }

    pub fn entry<K: IntoHeaderName>(&mut self, key: K) -> header::Entry<'_> {
        match self.inner.entry(key.into_header_name()) {
            btree_map::Entry::Occupied(entry) => {
                header::Entry::Occupied(header::OccupiedEntry { entry })
            }
            btree_map::Entry::Vacant(entry) => header::Entry::Vacant(header::VacantEntry { entry }),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &HeaderName> {
        self.inner.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.inner.iter()
    }

    pub fn extend(&mut self, other: HeaderMap) {
        self.inner.extend(other.inner);
    }
}

impl fmt::Debug for HeaderMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.inner.iter()).finish()
    }
}

impl<K: IntoHeaderName> Index<K> for HeaderMap {
    type Output = HeaderValue;

    fn index(&self, index: K) -> &Self::Output {
        self.get(index).expect("missing HTTP header")
    }
}

impl IntoIterator for HeaderMap {
    type Item = (HeaderName, HeaderValue);
    type IntoIter = btree_map::IntoIter<HeaderName, HeaderValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a HeaderMap {
    type Item = (&'a HeaderName, &'a HeaderValue);
    type IntoIter = btree_map::Iter<'a, HeaderName, HeaderValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum TokenRepr {
    Static(&'static str),
    Owned(String),
}

impl TokenRepr {
    fn as_str(&self) -> &str {
        match self {
            Self::Static(value) => value,
            Self::Owned(value) => value.as_str(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Method(TokenRepr);

impl Method {
    pub const GET: Self = Self(TokenRepr::Static("GET"));
    pub const POST: Self = Self(TokenRepr::Static("POST"));
    pub const PUT: Self = Self(TokenRepr::Static("PUT"));
    pub const PATCH: Self = Self(TokenRepr::Static("PATCH"));
    pub const DELETE: Self = Self(TokenRepr::Static("DELETE"));
    pub const HEAD: Self = Self(TokenRepr::Static("HEAD"));
    pub const OPTIONS: Self = Self(TokenRepr::Static("OPTIONS"));
    pub const CONNECT: Self = Self(TokenRepr::Static("CONNECT"));
    pub const TRACE: Self = Self(TokenRepr::Static("TRACE"));

    pub fn from_bytes(value: &[u8]) -> Result<Self> {
        let value = std::str::from_utf8(value).map_err(|_| Error::new("invalid method utf-8"))?;
        if value.is_empty() || value.bytes().any(|byte| !byte.is_ascii_graphic()) {
            return Err(Error::new("invalid method"));
        }
        Ok(match value {
            "GET" => Self::GET,
            "POST" => Self::POST,
            "PUT" => Self::PUT,
            "PATCH" => Self::PATCH,
            "DELETE" => Self::DELETE,
            "HEAD" => Self::HEAD,
            "OPTIONS" => Self::OPTIONS,
            "CONNECT" => Self::CONNECT,
            "TRACE" => Self::TRACE,
            other => Self(TokenRepr::Owned(other.to_string())),
        })
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<&str> for Method {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        Self::from_bytes(value.as_bytes())
    }
}

impl TryFrom<String> for Method {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::try_from(value.as_str())
    }
}

impl fmt::Debug for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq<&str> for Method {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<str> for Method {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<Method> for &str {
    fn eq(&self, other: &Method) -> bool {
        *self == other.as_str()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StatusCode(u16);

impl StatusCode {
    pub const SWITCHING_PROTOCOLS: Self = Self(101);
    pub const OK: Self = Self(200);
    pub const BAD_REQUEST: Self = Self(400);
    pub const UNAUTHORIZED: Self = Self(401);
    pub const FORBIDDEN: Self = Self(403);
    pub const NOT_FOUND: Self = Self(404);
    pub const TOO_MANY_REQUESTS: Self = Self(429);
    pub const INTERNAL_SERVER_ERROR: Self = Self(500);
    pub const BAD_GATEWAY: Self = Self(502);
    pub const SERVICE_UNAVAILABLE: Self = Self(503);
    pub const UPGRADE_REQUIRED: Self = Self(426);

    pub fn from_u16(value: u16) -> Result<Self> {
        if (100..=999).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::new("invalid status code"))
        }
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.0)
    }

    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.0)
    }

    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.0)
    }

    pub fn as_str(&self) -> &'static str {
        match self.0 {
            101 => "101",
            200 => "200",
            400 => "400",
            401 => "401",
            403 => "403",
            404 => "404",
            426 => "426",
            429 => "429",
            500 => "500",
            502 => "502",
            503 => "503",
            _ => "000",
        }
    }

    pub fn canonical_reason(&self) -> Option<&'static str> {
        Some(match self.0 {
            101 => "Switching Protocols",
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            426 => "Upgrade Required",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => return None,
        })
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.canonical_reason() {
            Some(reason) => write!(f, "{} {reason}", self.0),
            None => write!(f, "{}", self.0),
        }
    }
}

#[derive(Default)]
pub struct Extensions {
    inner: BTreeMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Extensions {
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) {
        self.inner.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.inner
            .get(&TypeId::of::<T>())
            .and_then(|value| value.downcast_ref())
    }
}

impl fmt::Debug for Extensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Extensions")
            .field("len", &self.inner.len())
            .finish()
    }
}

pub struct Response<T> {
    status: StatusCode,
    headers: HeaderMap,
    extensions: Extensions,
    body: T,
}

impl<T> Response<T> {
    pub fn builder() -> response::Builder {
        response::Builder::new()
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn body(&self) -> &T {
        &self.body
    }

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn into_body(self) -> T {
        self.body
    }
}

impl<T: fmt::Debug> fmt::Debug for Response<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .field("extensions", &self.extensions)
            .field("body", &self.body)
            .finish()
    }
}

pub struct Request<T> {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    extensions: Extensions,
    body: T,
}

impl<T> Request<T> {
    pub fn builder() -> request::Builder {
        request::Builder::new()
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn into_body(self) -> T {
        self.body
    }
}

impl<T: fmt::Debug> fmt::Debug for Request<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", &self.method)
            .field("uri", &self.uri)
            .field("headers", &self.headers)
            .field("extensions", &self.extensions)
            .field("body", &self.body)
            .finish()
    }
}

pub mod request {
    use super::*;

    #[derive(Default)]
    pub struct Builder {
        method: Option<Method>,
        uri: Option<Uri>,
        headers: HeaderMap,
        extensions: Extensions,
        valid: bool,
    }

    impl Builder {
        pub fn new() -> Self {
            Self {
                method: Some(Method::GET),
                uri: None,
                headers: HeaderMap::new(),
                extensions: Extensions::default(),
                valid: true,
            }
        }

        pub fn method<M>(mut self, method: M) -> Self
        where
            Method: TryFrom<M>,
        {
            match Method::try_from(method) {
                Ok(method) => self.method = Some(method),
                Err(_) => self.valid = false,
            }
            self
        }

        pub fn uri<U>(mut self, uri: U) -> Self
        where
            Uri: TryFrom<U>,
        {
            match Uri::try_from(uri) {
                Ok(uri) => self.uri = Some(uri),
                Err(_) => self.valid = false,
            }
            self
        }

        pub fn header<K, V>(mut self, key: K, value: V) -> Self
        where
            K: IntoHeaderName,
            HeaderValue: TryFrom<V>,
        {
            match HeaderValue::try_from(value) {
                Ok(value) => {
                    self.headers.insert(key, value);
                }
                Err(_) => self.valid = false,
            }
            self
        }

        pub fn extension<T: Any + Send + Sync>(mut self, value: T) -> Self {
            self.extensions.insert(value);
            self
        }

        pub fn body<T>(self, body: T) -> Result<Request<T>> {
            if !self.valid {
                return Err(Error::new("invalid request"));
            }
            Ok(Request {
                method: self.method.ok_or_else(|| Error::new("missing method"))?,
                uri: self.uri.ok_or_else(|| Error::new("missing uri"))?,
                headers: self.headers,
                extensions: self.extensions,
                body,
            })
        }
    }
}

pub mod response {
    use super::*;

    #[derive(Default)]
    pub struct Builder {
        status: Option<StatusCode>,
        headers: HeaderMap,
        extensions: Extensions,
    }

    impl Builder {
        pub fn new() -> Self {
            Self {
                status: Some(StatusCode::OK),
                headers: HeaderMap::new(),
                extensions: Extensions::default(),
            }
        }

        pub fn status(mut self, status: StatusCode) -> Self {
            self.status = Some(status);
            self
        }

        pub fn header<K, V>(mut self, key: K, value: V) -> Self
        where
            K: IntoHeaderName,
            HeaderValue: TryFrom<V>,
            <HeaderValue as TryFrom<V>>::Error: Into<Error>,
        {
            match HeaderValue::try_from(value) {
                Ok(value) => {
                    self.headers.insert(key, value);
                }
                Err(error) => {
                    self.status = None;
                    let _: Error = error.into();
                }
            }
            self
        }

        pub fn extension<T: Any + Send + Sync>(mut self, value: T) -> Self {
            self.extensions.insert(value);
            self
        }

        pub fn body<T>(self, body: T) -> Result<Response<T>> {
            Ok(Response {
                status: self.status.ok_or_else(|| Error::new("invalid response"))?,
                headers: self.headers,
                extensions: self.extensions,
                body,
            })
        }
    }
}

pub mod header {
    pub use crate::{HeaderMap, HeaderName, HeaderValue};

    use super::*;
    use std::collections::btree_map;

    pub const ACCEPT: HeaderName = HeaderName::from_static("accept");
    pub const AUTHORIZATION: HeaderName = HeaderName::from_static("authorization");
    pub const CONTENT_ENCODING: HeaderName = HeaderName::from_static("content-encoding");
    pub const CONTENT_LENGTH: HeaderName = HeaderName::from_static("content-length");
    pub const CONTENT_TYPE: HeaderName = HeaderName::from_static("content-type");
    pub const COOKIE: HeaderName = HeaderName::from_static("cookie");
    pub const ETAG: HeaderName = HeaderName::from_static("etag");
    pub const LOCATION: HeaderName = HeaderName::from_static("location");
    pub const SET_COOKIE: HeaderName = HeaderName::from_static("set-cookie");

    pub enum Entry<'a> {
        Occupied(OccupiedEntry<'a>),
        Vacant(VacantEntry<'a>),
    }

    pub struct OccupiedEntry<'a> {
        pub(crate) entry: btree_map::OccupiedEntry<'a, HeaderName, HeaderValue>,
    }

    impl OccupiedEntry<'_> {
        pub fn get(&self) -> &HeaderValue {
            self.entry.get()
        }

        pub fn insert(&mut self, value: HeaderValue) -> HeaderValue {
            self.entry.insert(value)
        }
    }

    pub struct VacantEntry<'a> {
        pub(crate) entry: btree_map::VacantEntry<'a, HeaderName, HeaderValue>,
    }

    impl<'a> VacantEntry<'a> {
        pub fn insert(self, value: HeaderValue) -> &'a mut HeaderValue {
            self.entry.insert(value)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Uri(String);

impl Uri {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn scheme_str(&self) -> Option<&str> {
        self.0.split_once("://").map(|(scheme, _)| scheme)
    }

    pub fn authority(&self) -> Option<Authority<'_>> {
        let after_scheme = self.0.split_once("://")?.1;
        let end = after_scheme
            .find(['/', '?', '#'])
            .unwrap_or(after_scheme.len());
        Some(Authority(&after_scheme[..end]))
    }

    pub fn host(&self) -> Option<&str> {
        let authority = self.authority()?;
        let authority = authority.as_str();
        let host_port = authority
            .rsplit_once('@')
            .map(|(_, host)| host)
            .unwrap_or(authority);
        Some(
            host_port
                .split_once(':')
                .map(|(host, _)| host)
                .unwrap_or(host_port),
        )
        .filter(|host| !host.is_empty())
    }

    pub fn port_u16(&self) -> Option<u16> {
        let authority = self.authority()?;
        let authority = authority.as_str();
        let host_port = authority
            .rsplit_once('@')
            .map(|(_, host)| host)
            .unwrap_or(authority);
        host_port
            .rsplit_once(':')
            .and_then(|(_, port)| port.parse::<u16>().ok())
    }

    pub fn path_and_query(&self) -> &str {
        let Some((_, after_scheme)) = self.0.split_once("://") else {
            return self.as_str();
        };
        let Some(index) = after_scheme.find(['/', '?']) else {
            return "/";
        };
        &after_scheme[index..]
    }
}

pub struct Authority<'a>(&'a str);

impl<'a> Authority<'a> {
    pub fn as_str(&self) -> &'a str {
        self.0
    }
}

impl FromStr for Uri {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        if value.is_empty() {
            Err(Error::new("empty uri"))
        } else {
            Ok(Self(value.to_string()))
        }
    }
}

impl TryFrom<String> for Uri {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::from_str(&value)
    }
}

impl TryFrom<&String> for Uri {
    type Error = Error;

    fn try_from(value: &String) -> Result<Self> {
        Self::from_str(value)
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
