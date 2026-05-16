//! Compatibility surface for crates that currently use `tungstenite`.
//!
//! This crate owns the public protocol types used by Edgerun Codex. The current
//! backend adapter still converts to the upstream fork internally while we fill
//! in the Edgerun WebSocket implementation behind this boundary.

use bytes::Bytes;
use bytes::BytesMut;
use std::borrow::Borrow;
use std::fmt;
use std::io;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::ops::Deref;
#[cfg(feature = "stream")]
use std::pin::Pin;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
#[cfg(feature = "stream")]
use std::task::{Context, Poll};

#[cfg(feature = "stream")]
use edgerun_futures::Sink;
#[cfg(feature = "stream")]
use edgerun_futures::Stream as FuturesStream;

#[cfg(feature = "handshake")]
pub mod http {
    pub use crate::backend::http::*;
}

pub mod client {
    use crate::Error;
    use crate::Result;
    use crate::backend::http;
    use crate::backend::http::Request as HttpRequest;
    use crate::backend::http::Uri;
    use edgerun_encoding::base64::standard_encode;
    use http::HeaderName;

    pub type ClientRequest = HttpRequest<()>;
    pub type Request = ClientRequest;
    pub type Response = http::Response<Option<Vec<u8>>>;

    pub trait IntoClientRequest {
        fn into_client_request(self) -> Result<ClientRequest>;
    }

    impl IntoClientRequest for &str {
        fn into_client_request(self) -> Result<ClientRequest> {
            self.parse::<Uri>()
                .map_err(|error| Error::Other(format!("invalid websocket URI: {error}")))?
                .into_client_request()
        }
    }

    impl IntoClientRequest for &String {
        fn into_client_request(self) -> Result<ClientRequest> {
            self.as_str().into_client_request()
        }
    }

    impl IntoClientRequest for String {
        fn into_client_request(self) -> Result<ClientRequest> {
            self.as_str().into_client_request()
        }
    }

    impl IntoClientRequest for &Uri {
        fn into_client_request(self) -> Result<ClientRequest> {
            self.clone().into_client_request()
        }
    }

    impl IntoClientRequest for Uri {
        fn into_client_request(self) -> Result<ClientRequest> {
            let authority = self
                .authority()
                .ok_or_else(|| Error::Other("websocket URI has no host".to_string()))?
                .as_str();
            let host = authority
                .find('@')
                .map(|index| &authority[index + 1..])
                .unwrap_or(authority);

            if host.is_empty() {
                return Err(Error::Other("websocket URI has an empty host".to_string()));
            }

            HttpRequest::<()>::builder()
                .method("GET")
                .header("Host", host)
                .header("Connection", "Upgrade")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", generate_key())
                .uri(self)
                .body(())
                .map_err(|error| {
                    Error::Other(format!("failed to build websocket request: {error}"))
                })
        }
    }

    impl IntoClientRequest for ClientRequest {
        fn into_client_request(self) -> Result<ClientRequest> {
            Ok(self)
        }
    }

    #[derive(Debug, Clone)]
    pub struct ClientRequestBuilder {
        uri: Uri,
        additional_headers: Vec<(String, String)>,
        subprotocols: Vec<String>,
    }

    impl ClientRequestBuilder {
        #[must_use]
        pub const fn new(uri: Uri) -> Self {
            Self {
                uri,
                additional_headers: Vec::new(),
                subprotocols: Vec::new(),
            }
        }

        #[must_use]
        pub fn with_header<K, V>(mut self, key: K, value: V) -> Self
        where
            K: Into<String>,
            V: Into<String>,
        {
            self.additional_headers.push((key.into(), value.into()));
            self
        }

        #[must_use]
        pub fn with_sub_protocol<P>(mut self, protocol: P) -> Self
        where
            P: Into<String>,
        {
            self.subprotocols.push(protocol.into());
            self
        }
    }

    impl IntoClientRequest for ClientRequestBuilder {
        fn into_client_request(self) -> Result<ClientRequest> {
            let mut request = self.uri.into_client_request()?;
            let headers = request.headers_mut();

            for (key, value) in self.additional_headers {
                let key = HeaderName::try_from(key)
                    .map_err(|error| Error::Other(format!("invalid websocket header: {error}")))?;
                let value = value.parse().map_err(|error| {
                    Error::Other(format!("invalid websocket header value: {error}"))
                })?;
                headers.append(key, value);
            }

            if !self.subprotocols.is_empty() {
                let protocols = self.subprotocols.join(", ").parse().map_err(|error| {
                    Error::Other(format!("invalid websocket subprotocol value: {error}"))
                })?;
                headers.append("Sec-WebSocket-Protocol", protocols);
            }

            Ok(request)
        }
    }

    pub fn generate_key() -> String {
        let mut bytes = [0u8; 16];
        edgerun_crypto::fill_random(&mut bytes).expect("edgerun secure random source unavailable");
        standard_encode(&bytes)
    }

    #[cfg(feature = "handshake")]
    pub fn client<S, R>(request: R, stream: S) -> Result<(crate::WebSocket<S>, Response)>
    where
        S: std::io::Read + std::io::Write,
        R: IntoClientRequest,
    {
        crate::client(request, stream)
    }

    #[cfg(feature = "handshake")]
    pub fn client_with_config<S, R>(
        request: R,
        stream: S,
        config: Option<crate::protocol::WebSocketConfig>,
    ) -> Result<(crate::WebSocket<S>, Response)>
    where
        S: std::io::Read + std::io::Write,
        R: IntoClientRequest,
    {
        crate::client_with_config(request, stream, config)
    }

    #[cfg(feature = "handshake")]
    pub fn connect<R>(
        request: R,
    ) -> Result<(
        crate::WebSocket<crate::MaybeTlsStream<std::net::TcpStream>>,
        Response,
    )>
    where
        R: IntoClientRequest,
    {
        crate::connect(request)
    }

    #[cfg(feature = "handshake")]
    pub fn connect_with_config<R>(
        request: R,
        config: Option<crate::protocol::WebSocketConfig>,
        max_redirects: u8,
    ) -> Result<(
        crate::WebSocket<crate::MaybeTlsStream<std::net::TcpStream>>,
        Response,
    )>
    where
        R: IntoClientRequest,
    {
        crate::connect_with_config(request, config, max_redirects)
    }

    #[cfg(feature = "handshake")]
    pub fn uri_mode(uri: &http::Uri) -> Result<crate::Mode> {
        crate::uri_mode(uri)
    }
}

pub use client::ClientRequestBuilder;

#[non_exhaustive]
#[allow(missing_debug_implementations)]
pub enum Connector {
    Plain,
}

pub mod stream {
    use std::fmt;
    use std::io::Read;
    use std::io::Result as IoResult;
    use std::io::Write;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Mode {
        Plain,
        Tls,
    }

    impl From<Mode> for crate::backend::stream::Mode {
        fn from(value: Mode) -> Self {
            match value {
                Mode::Plain => Self::Plain,
                Mode::Tls => Self::Tls,
            }
        }
    }

    impl From<crate::backend::stream::Mode> for Mode {
        fn from(value: crate::backend::stream::Mode) -> Self {
            match value {
                crate::backend::stream::Mode::Plain => Self::Plain,
                crate::backend::stream::Mode::Tls => Self::Tls,
            }
        }
    }

    pub trait MaybeTlsStreamIo: Read + Write + Send + Unpin {}

    impl<T> MaybeTlsStreamIo for T where T: Read + Write + Send + Unpin {}

    #[non_exhaustive]
    #[allow(clippy::large_enum_variant)]
    pub enum MaybeTlsStream<S: Read + Write> {
        Plain(S),
        Tls(Box<dyn MaybeTlsStreamIo>),
    }

    impl<S> MaybeTlsStream<S>
    where
        S: Read + Write,
    {
        pub(crate) fn from_backend(stream: crate::backend::stream::MaybeTlsStream<S>) -> Self
        where
            crate::backend::stream::MaybeTlsStream<S>: MaybeTlsStreamIo + 'static,
        {
            match stream {
                crate::backend::stream::MaybeTlsStream::Plain(stream) => Self::Plain(stream),
            }
        }

        pub fn get_ref(&self) -> &S {
            match self {
                Self::Plain(stream) => stream,
                Self::Tls(_) => {
                    panic!("TLS stream does not expose an Edgerun plain stream reference")
                }
            }
        }

        pub fn get_mut(&mut self) -> &mut S {
            match self {
                Self::Plain(stream) => stream,
                Self::Tls(_) => {
                    panic!("TLS stream does not expose an Edgerun plain stream reference")
                }
            }
        }
    }

    impl<S> fmt::Debug for MaybeTlsStream<S>
    where
        S: Read + Write + fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Plain(stream) => f
                    .debug_tuple("MaybeTlsStream::Plain")
                    .field(stream)
                    .finish(),
                Self::Tls(_) => f.debug_tuple("MaybeTlsStream::Tls").finish(),
            }
        }
    }

    impl<S> Read for MaybeTlsStream<S>
    where
        S: Read + Write,
    {
        fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
            match self {
                Self::Plain(stream) => stream.read(buf),
                Self::Tls(stream) => stream.read(buf),
            }
        }
    }

    impl<S> Write for MaybeTlsStream<S>
    where
        S: Read + Write,
    {
        fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
            match self {
                Self::Plain(stream) => stream.write(buf),
                Self::Tls(stream) => stream.write(buf),
            }
        }

        fn flush(&mut self) -> IoResult<()> {
            match self {
                Self::Plain(stream) => stream.flush(),
                Self::Tls(stream) => stream.flush(),
            }
        }
    }

    pub trait MaybeTlsStreamExt<S>
    where
        S: Read + Write,
    {
        fn is_plain(&self) -> bool;
        fn plain_ref(&self) -> Option<&S>;
        fn plain_mut(&mut self) -> Option<&mut S>;
    }

    impl<S> MaybeTlsStreamExt<S> for MaybeTlsStream<S>
    where
        S: Read + Write,
    {
        fn is_plain(&self) -> bool {
            matches!(self, MaybeTlsStream::Plain(_))
        }

        fn plain_ref(&self) -> Option<&S> {
            match self {
                MaybeTlsStream::Plain(stream) => Some(stream),
                _ => None,
            }
        }

        fn plain_mut(&mut self) -> Option<&mut S> {
            match self {
                MaybeTlsStream::Plain(stream) => Some(stream),
                _ => None,
            }
        }
    }
}

pub use stream::MaybeTlsStream;
pub use stream::MaybeTlsStreamExt;
pub use stream::Mode;

pub mod handshake {
    pub mod client {
        use crate::backend::http;
        pub use crate::client::generate_key;

        pub type Request = http::Request<()>;
        pub type Response = http::Response<Option<Vec<u8>>>;
    }

    pub mod server {
        use crate::backend::http;

        pub type ErrorResponse = http::Response<Option<String>>;
        pub type Request = http::Request<()>;
        pub type Response = http::Response<()>;

        #[cfg(feature = "handshake")]
        pub fn create_response(request: &Request) -> crate::Result<Response> {
            crate::backend::handshake::server::create_response(request).map_err(crate::Error::from)
        }

        #[cfg(feature = "handshake")]
        pub fn create_response_with_body<T1, T2>(
            request: &http::Request<T1>,
            generate_body: impl FnOnce() -> T2,
        ) -> crate::Result<http::Response<T2>> {
            crate::backend::handshake::server::create_response_with_body(request, generate_body)
                .map_err(crate::Error::from)
        }

        #[cfg(feature = "handshake")]
        pub fn write_response<T>(
            writer: impl std::io::Write,
            response: &http::Response<T>,
        ) -> crate::Result<()> {
            crate::backend::handshake::server::write_response(writer, response)
                .map_err(crate::Error::from)
        }

        pub trait Callback: Sized {
            fn on_request(
                self,
                request: &Request,
                response: Response,
            ) -> std::result::Result<Response, ErrorResponse>;
        }

        impl<F> Callback for F
        where
            F: FnOnce(&Request, Response) -> std::result::Result<Response, ErrorResponse>,
        {
            fn on_request(
                self,
                request: &Request,
                response: Response,
            ) -> std::result::Result<Response, ErrorResponse> {
                self(request, response)
            }
        }

        #[derive(Clone, Copy, Debug)]
        pub struct NoCallback;

        impl Callback for NoCallback {
            fn on_request(
                self,
                _request: &Request,
                response: Response,
            ) -> std::result::Result<Response, ErrorResponse> {
                Ok(response)
            }
        }
    }
}

pub mod extensions {
    #[derive(Copy, Clone, Debug, Default)]
    #[non_exhaustive]
    pub struct ExtensionsConfig {
        pub permessage_deflate: Option<compression::deflate::DeflateConfig>,
    }

    pub mod compression {
        pub mod deflate {
            #[derive(Copy, Clone, Debug, Default)]
            #[non_exhaustive]
            pub struct DeflateConfig;
        }
    }
}

pub mod protocol {
    pub use crate::CloseFrame;
    pub use crate::Message;
    pub use crate::extensions::ExtensionsConfig;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Role {
        Server,
        Client,
    }

    impl From<Role> for crate::backend::protocol::Role {
        fn from(value: Role) -> Self {
            match value {
                Role::Server => Self::Server,
                Role::Client => Self::Client,
            }
        }
    }

    impl From<crate::backend::protocol::Role> for Role {
        fn from(value: crate::backend::protocol::Role) -> Self {
            match value {
                crate::backend::protocol::Role::Server => Self::Server,
                crate::backend::protocol::Role::Client => Self::Client,
            }
        }
    }

    pub mod frame {
        pub use crate::CloseFrame;
        pub use crate::Frame;
        pub use crate::FrameHeader;
        pub use crate::Utf8Bytes;

        pub mod coding {
            pub use crate::CloseCode;
            pub use crate::Control;
            pub use crate::Data;
            pub use crate::OpCode;
        }
    }

    #[derive(Debug, Clone, Copy)]
    #[non_exhaustive]
    pub struct WebSocketConfig {
        pub read_buffer_size: usize,
        pub write_buffer_size: usize,
        pub max_write_buffer_size: usize,
        pub max_message_size: Option<usize>,
        pub max_frame_size: Option<usize>,
        pub accept_unmasked_frames: bool,
        pub extensions: ExtensionsConfig,
    }

    impl Default for WebSocketConfig {
        fn default() -> Self {
            Self {
                read_buffer_size: 128 * 1024,
                write_buffer_size: 128 * 1024,
                max_write_buffer_size: usize::MAX,
                max_message_size: Some(64 << 20),
                max_frame_size: Some(16 << 20),
                accept_unmasked_frames: false,
                extensions: ExtensionsConfig::default(),
            }
        }
    }

    impl WebSocketConfig {
        pub fn read_buffer_size(mut self, read_buffer_size: usize) -> Self {
            self.read_buffer_size = read_buffer_size;
            self
        }

        pub fn write_buffer_size(mut self, write_buffer_size: usize) -> Self {
            self.write_buffer_size = write_buffer_size;
            self
        }

        pub fn max_write_buffer_size(mut self, max_write_buffer_size: usize) -> Self {
            self.max_write_buffer_size = max_write_buffer_size;
            self
        }

        pub fn max_message_size(mut self, max_message_size: Option<usize>) -> Self {
            self.max_message_size = max_message_size;
            self
        }

        pub fn max_frame_size(mut self, max_frame_size: Option<usize>) -> Self {
            self.max_frame_size = max_frame_size;
            self
        }

        pub fn accept_unmasked_frames(mut self, accept_unmasked_frames: bool) -> Self {
            self.accept_unmasked_frames = accept_unmasked_frames;
            self
        }
    }

    impl From<WebSocketConfig> for crate::backend::WebSocketConfig {
        fn from(value: WebSocketConfig) -> Self {
            let mut config = crate::backend::WebSocketConfig::default()
                .read_buffer_size(value.read_buffer_size)
                .write_buffer_size(value.write_buffer_size)
                .max_write_buffer_size(value.max_write_buffer_size)
                .max_message_size(value.max_message_size)
                .max_frame_size(value.max_frame_size)
                .accept_unmasked_frames(value.accept_unmasked_frames);
            config.extensions = value.extensions.into();
            config
        }
    }

    impl From<&crate::backend::WebSocketConfig> for WebSocketConfig {
        fn from(value: &crate::backend::WebSocketConfig) -> Self {
            Self {
                read_buffer_size: value.read_buffer_size,
                write_buffer_size: value.write_buffer_size,
                max_write_buffer_size: value.max_write_buffer_size,
                max_message_size: value.max_message_size,
                max_frame_size: value.max_frame_size,
                accept_unmasked_frames: value.accept_unmasked_frames,
                extensions: (&value.extensions).into(),
            }
        }
    }
}

impl From<&crate::backend::extensions::ExtensionsConfig> for extensions::ExtensionsConfig {
    fn from(value: &crate::backend::extensions::ExtensionsConfig) -> Self {
        let mut config = extensions::ExtensionsConfig::default();
        #[cfg(feature = "deflate")]
        {
            config.permessage_deflate = value
                .permessage_deflate
                .map(|_| extensions::compression::deflate::DeflateConfig);
        }
        #[cfg(not(feature = "deflate"))]
        {
            let _ = value;
        }
        config
    }
}

impl From<extensions::ExtensionsConfig> for crate::backend::extensions::ExtensionsConfig {
    fn from(value: extensions::ExtensionsConfig) -> Self {
        let mut config = crate::backend::extensions::ExtensionsConfig::default();
        #[cfg(feature = "deflate")]
        {
            config.permessage_deflate = value.permessage_deflate.map(|_| {
                crate::backend::extensions::compression::deflate::DeflateConfig::default()
            });
        }
        #[cfg(not(feature = "deflate"))]
        {
            let _ = value;
        }
        config
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Utf8Bytes(String);

impl Utf8Bytes {
    pub fn from_static(value: &'static str) -> Self {
        Self(value.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub unsafe fn from_bytes_unchecked(bytes: Bytes) -> Self {
        Self(unsafe { String::from_utf8_unchecked(bytes.to_vec()) })
    }
}

impl Deref for Utf8Bytes {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for Utf8Bytes {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for Utf8Bytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Borrow<str> for Utf8Bytes {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Utf8Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for Utf8Bytes {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Utf8Bytes {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<&String> for Utf8Bytes {
    fn from(value: &String) -> Self {
        Self(value.clone())
    }
}

impl From<Utf8Bytes> for String {
    fn from(value: Utf8Bytes) -> Self {
        value.0
    }
}

impl From<Utf8Bytes> for Bytes {
    fn from(value: Utf8Bytes) -> Self {
        value.0.into()
    }
}

impl TryFrom<Bytes> for Utf8Bytes {
    type Error = FromUtf8Error;

    fn try_from(value: Bytes) -> std::result::Result<Self, Self::Error> {
        String::from_utf8(value.to_vec()).map(Self)
    }
}

impl TryFrom<BytesMut> for Utf8Bytes {
    type Error = FromUtf8Error;

    fn try_from(value: BytesMut) -> std::result::Result<Self, Self::Error> {
        value.freeze().try_into()
    }
}

impl TryFrom<Vec<u8>> for Utf8Bytes {
    type Error = FromUtf8Error;

    fn try_from(value: Vec<u8>) -> std::result::Result<Self, Self::Error> {
        String::from_utf8(value).map(Self)
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum CloseCode {
    Normal,
    Away,
    Protocol,
    Unsupported,
    Status,
    Abnormal,
    Invalid,
    Policy,
    Size,
    Extension,
    Error,
    Restart,
    Again,
    #[doc(hidden)]
    Tls,
    #[doc(hidden)]
    Reserved(u16),
    #[doc(hidden)]
    Iana(u16),
    #[doc(hidden)]
    Library(u16),
    #[doc(hidden)]
    Bad(u16),
}

impl CloseCode {
    pub fn is_allowed(self) -> bool {
        !matches!(
            self,
            Self::Bad(_) | Self::Reserved(_) | Self::Status | Self::Abnormal | Self::Tls
        )
    }
}

impl fmt::Display for CloseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code: u16 = self.into();
        write!(f, "{code}")
    }
}

impl From<CloseCode> for u16 {
    fn from(code: CloseCode) -> Self {
        match code {
            CloseCode::Normal => 1000,
            CloseCode::Away => 1001,
            CloseCode::Protocol => 1002,
            CloseCode::Unsupported => 1003,
            CloseCode::Status => 1005,
            CloseCode::Abnormal => 1006,
            CloseCode::Invalid => 1007,
            CloseCode::Policy => 1008,
            CloseCode::Size => 1009,
            CloseCode::Extension => 1010,
            CloseCode::Error => 1011,
            CloseCode::Restart => 1012,
            CloseCode::Again => 1013,
            CloseCode::Tls => 1015,
            CloseCode::Reserved(code) => code,
            CloseCode::Iana(code) => code,
            CloseCode::Library(code) => code,
            CloseCode::Bad(code) => code,
        }
    }
}

impl From<&CloseCode> for u16 {
    fn from(code: &CloseCode) -> Self {
        (*code).into()
    }
}

impl From<u16> for CloseCode {
    fn from(code: u16) -> Self {
        match code {
            1000 => CloseCode::Normal,
            1001 => CloseCode::Away,
            1002 => CloseCode::Protocol,
            1003 => CloseCode::Unsupported,
            1005 => CloseCode::Status,
            1006 => CloseCode::Abnormal,
            1007 => CloseCode::Invalid,
            1008 => CloseCode::Policy,
            1009 => CloseCode::Size,
            1010 => CloseCode::Extension,
            1011 => CloseCode::Error,
            1012 => CloseCode::Restart,
            1013 => CloseCode::Again,
            1015 => CloseCode::Tls,
            1..=999 => CloseCode::Bad(code),
            1016..=2999 => CloseCode::Reserved(code),
            3000..=3999 => CloseCode::Iana(code),
            4000..=4999 => CloseCode::Library(code),
            _ => CloseCode::Bad(code),
        }
    }
}

impl From<CloseCode> for crate::backend::CloseCode {
    fn from(code: CloseCode) -> Self {
        u16::from(code).into()
    }
}

impl From<crate::backend::CloseCode> for CloseCode {
    fn from(code: crate::backend::CloseCode) -> Self {
        u16::from(code).into()
    }
}

#[derive(Debug, Clone)]
pub struct CloseFrame {
    pub code: CloseCode,
    pub reason: Utf8Bytes,
}

impl fmt::Display for CloseFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.reason, self.code)
    }
}

impl From<CloseFrame> for crate::backend::CloseFrame {
    fn from(value: CloseFrame) -> Self {
        Self {
            code: value.code.into(),
            reason: String::from(value.reason).into(),
        }
    }
}

impl From<crate::backend::CloseFrame> for CloseFrame {
    fn from(value: crate::backend::CloseFrame) -> Self {
        Self {
            code: value.code.into(),
            reason: value.reason.to_string().into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Data {
    Continue,
    Text,
    Binary,
    Reserved(u8),
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Continue => write!(f, "CONTINUE"),
            Self::Text => write!(f, "TEXT"),
            Self::Binary => write!(f, "BINARY"),
            Self::Reserved(code) => write!(f, "RESERVED_DATA_{code}"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Control {
    Close,
    Ping,
    Pong,
    Reserved(u8),
}

impl fmt::Display for Control {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Close => write!(f, "CLOSE"),
            Self::Ping => write!(f, "PING"),
            Self::Pong => write!(f, "PONG"),
            Self::Reserved(code) => write!(f, "RESERVED_CONTROL_{code}"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OpCode {
    Data(Data),
    Control(Control),
}

impl fmt::Display for OpCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data(data) => data.fmt(f),
            Self::Control(control) => control.fmt(f),
        }
    }
}

impl From<OpCode> for u8 {
    fn from(code: OpCode) -> Self {
        match code {
            OpCode::Data(Data::Continue) => 0,
            OpCode::Data(Data::Text) => 1,
            OpCode::Data(Data::Binary) => 2,
            OpCode::Data(Data::Reserved(code)) => code,
            OpCode::Control(Control::Close) => 8,
            OpCode::Control(Control::Ping) => 9,
            OpCode::Control(Control::Pong) => 10,
            OpCode::Control(Control::Reserved(code)) => code,
        }
    }
}

impl From<u8> for OpCode {
    fn from(code: u8) -> Self {
        match code {
            0 => Self::Data(Data::Continue),
            1 => Self::Data(Data::Text),
            2 => Self::Data(Data::Binary),
            code @ 3..=7 => Self::Data(Data::Reserved(code)),
            8 => Self::Control(Control::Close),
            9 => Self::Control(Control::Ping),
            10 => Self::Control(Control::Pong),
            code @ 11..=15 => Self::Control(Control::Reserved(code)),
            _ => panic!("Bug: OpCode out of range"),
        }
    }
}

impl From<OpCode> for crate::backend::protocol::frame::coding::OpCode {
    fn from(value: OpCode) -> Self {
        u8::from(value).into()
    }
}

impl From<crate::backend::protocol::frame::coding::OpCode> for OpCode {
    fn from(value: crate::backend::protocol::frame::coding::OpCode) -> Self {
        u8::from(value).into()
    }
}

#[allow(missing_copy_implementations)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FrameHeader {
    pub is_final: bool,
    pub rsv1: bool,
    pub rsv2: bool,
    pub rsv3: bool,
    pub opcode: OpCode,
    pub mask: Option<[u8; 4]>,
}

impl Default for FrameHeader {
    fn default() -> Self {
        Self {
            is_final: true,
            rsv1: false,
            rsv2: false,
            rsv3: false,
            opcode: OpCode::Control(Control::Close),
            mask: None,
        }
    }
}

impl FrameHeader {
    pub fn parse(cursor: &mut Cursor<impl AsRef<[u8]>>) -> Result<Option<(Self, u64)>> {
        crate::backend::protocol::frame::FrameHeader::parse(cursor)
            .map(|option| option.map(|(header, length)| (header.into(), length)))
            .map_err(Error::from)
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self, length: u64) -> usize {
        crate::backend::protocol::frame::FrameHeader::from(self.clone()).len(length)
    }

    pub fn format(&self, length: u64, output: &mut impl Write) -> Result<()> {
        crate::backend::protocol::frame::FrameHeader::from(self.clone())
            .format(length, output)
            .map_err(Error::from)
    }
}

impl From<FrameHeader> for crate::backend::protocol::frame::FrameHeader {
    fn from(value: FrameHeader) -> Self {
        Self {
            is_final: value.is_final,
            rsv1: value.rsv1,
            rsv2: value.rsv2,
            rsv3: value.rsv3,
            opcode: value.opcode.into(),
            mask: value.mask,
        }
    }
}

impl From<crate::backend::protocol::frame::FrameHeader> for FrameHeader {
    fn from(value: crate::backend::protocol::frame::FrameHeader) -> Self {
        Self {
            is_final: value.is_final,
            rsv1: value.rsv1,
            rsv2: value.rsv2,
            rsv3: value.rsv3,
            opcode: value.opcode.into(),
            mask: value.mask,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    inner: crate::backend::Frame,
}

impl Frame {
    pub fn message(data: impl Into<Bytes>, opcode: OpCode, is_final: bool) -> Self {
        Self {
            inner: crate::backend::Frame::message(
                Vec::<u8>::from(data.into()),
                opcode.into(),
                is_final,
            ),
        }
    }

    pub fn pong(data: impl Into<Bytes>) -> Self {
        Self {
            inner: crate::backend::Frame::pong(Vec::<u8>::from(data.into())),
        }
    }

    pub fn ping(data: impl Into<Bytes>) -> Self {
        Self {
            inner: crate::backend::Frame::ping(Vec::<u8>::from(data.into())),
        }
    }

    pub fn close(message: Option<CloseFrame>) -> Self {
        Self {
            inner: crate::backend::Frame::close(message.map(Into::into)),
        }
    }

    pub fn from_payload(header: FrameHeader, payload: Bytes) -> Self {
        Self {
            inner: crate::backend::Frame::from_payload(
                header.into(),
                Vec::<u8>::from(payload).into(),
            ),
        }
    }

    pub fn format(self, output: &mut impl Write) -> Result<()> {
        self.inner.format(output).map_err(Error::from)
    }

    pub fn header(&self) -> FrameHeader {
        self.inner.header().clone().into()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn payload(&self) -> &[u8] {
        self.inner.payload()
    }

    pub fn into_payload(self) -> Bytes {
        self.inner.into_payload().to_vec().into()
    }

    pub fn into_text(self) -> Result<Utf8Bytes> {
        self.into_payload().try_into().map_err(Error::from)
    }

    pub fn to_text(&self) -> Result<&str> {
        std::str::from_utf8(self.payload()).map_err(Error::from)
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Text(Utf8Bytes),
    Binary(Bytes),
    Ping(Bytes),
    Pong(Bytes),
    Close(Option<CloseFrame>),
    Frame(Frame),
}

impl Message {
    pub fn text<S: Into<Utf8Bytes>>(value: S) -> Self {
        Self::Text(value.into())
    }

    pub fn binary<B: Into<Bytes>>(value: B) -> Self {
        Self::Binary(value.into())
    }

    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary(_))
    }

    pub fn is_ping(&self) -> bool {
        matches!(self, Self::Ping(_))
    }

    pub fn is_pong(&self) -> bool {
        matches!(self, Self::Pong(_))
    }

    pub fn is_close(&self) -> bool {
        matches!(self, Self::Close(_))
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Binary(data) | Self::Ping(data) | Self::Pong(data) => data.len(),
            Self::Close(frame) => frame.as_ref().map(|frame| frame.reason.len()).unwrap_or(0),
            Self::Frame(frame) => frame.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn into_data(self) -> Bytes {
        match self {
            Self::Text(text) => text.into(),
            Self::Binary(data) | Self::Ping(data) | Self::Pong(data) => data,
            Self::Close(None) => Bytes::new(),
            Self::Close(Some(frame)) => frame.reason.into(),
            Self::Frame(frame) => frame.into_payload(),
        }
    }

    pub fn into_text(self) -> Result<Utf8Bytes> {
        match self {
            Self::Text(text) => Ok(text),
            Self::Binary(data) | Self::Ping(data) | Self::Pong(data) => {
                data.try_into().map_err(Error::from)
            }
            Self::Close(None) => Ok(Utf8Bytes::default()),
            Self::Close(Some(frame)) => Ok(frame.reason),
            Self::Frame(frame) => frame.into_text(),
        }
    }

    pub fn to_text(&self) -> Result<&str> {
        match self {
            Self::Text(text) => Ok(text.as_str()),
            Self::Binary(data) | Self::Ping(data) | Self::Pong(data) => {
                std::str::from_utf8(data).map_err(Error::from)
            }
            Self::Close(None) => Ok(""),
            Self::Close(Some(frame)) => Ok(frame.reason.as_str()),
            Self::Frame(frame) => frame.to_text(),
        }
    }
}

impl From<String> for Message {
    fn from(value: String) -> Self {
        Self::text(value)
    }
}

impl From<&str> for Message {
    fn from(value: &str) -> Self {
        Self::text(value)
    }
}

impl From<&String> for Message {
    fn from(value: &String) -> Self {
        Self::text(value)
    }
}

impl From<Bytes> for Message {
    fn from(value: Bytes) -> Self {
        Self::binary(value)
    }
}

impl From<Vec<u8>> for Message {
    fn from(value: Vec<u8>) -> Self {
        Self::binary(value)
    }
}

impl From<&[u8]> for Message {
    fn from(value: &[u8]) -> Self {
        Self::binary(Bytes::copy_from_slice(value))
    }
}

impl From<Message> for crate::backend::Message {
    fn from(value: Message) -> Self {
        match value {
            Message::Text(text) => crate::backend::Message::Text(String::from(text).into()),
            Message::Binary(bytes) => {
                crate::backend::Message::Binary(Vec::<u8>::from(bytes).into())
            }
            Message::Ping(bytes) => crate::backend::Message::Ping(Vec::<u8>::from(bytes).into()),
            Message::Pong(bytes) => crate::backend::Message::Pong(Vec::<u8>::from(bytes).into()),
            Message::Close(frame) => crate::backend::Message::Close(frame.map(Into::into)),
            Message::Frame(frame) => crate::backend::Message::Frame(frame.inner),
        }
    }
}

impl From<crate::backend::Message> for Message {
    fn from(value: crate::backend::Message) -> Self {
        match value {
            crate::backend::Message::Text(text) => Message::Text(text.to_string().into()),
            crate::backend::Message::Binary(bytes) => Message::Binary(bytes.to_vec().into()),
            crate::backend::Message::Ping(bytes) => Message::Ping(bytes.to_vec().into()),
            crate::backend::Message::Pong(bytes) => Message::Pong(bytes.to_vec().into()),
            crate::backend::Message::Close(frame) => Message::Close(frame.map(Into::into)),
            crate::backend::Message::Frame(frame) => Message::Frame(Frame { inner: frame }),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    ConnectionClosed,
    AlreadyClosed,
    Io(io::Error),
    Tls(String),
    Capacity(String),
    Protocol(String),
    WriteBufferFull(Box<Message>),
    Utf8(String),
    AttackAttempt,
    Url(String),
    Http(Box<crate::backend::http::Response<Option<Vec<u8>>>>),
    HttpFormat(String),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ConnectionClosed => f.write_str("Connection closed normally"),
            Error::AlreadyClosed => f.write_str("Trying to work with closed connection"),
            Error::Io(error) => write!(f, "IO error: {error}"),
            Error::Tls(error) => write!(f, "TLS error: {error}"),
            Error::Capacity(error) => write!(f, "Space limit exceeded: {error}"),
            Error::Protocol(error) => write!(f, "WebSocket protocol error: {error}"),
            Error::WriteBufferFull(_) => f.write_str("Write buffer is full"),
            Error::Utf8(error) => write!(f, "UTF-8 encoding error: {error}"),
            Error::AttackAttempt => f.write_str("Attack attempt detected"),
            Error::Url(error) => write!(f, "URL error: {error}"),
            Error::Http(response) => write!(f, "HTTP error: {}", response.status()),
            Error::HttpFormat(error) => write!(f, "HTTP format error: {error}"),
            Error::Other(error) => f.write_str(error),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<Utf8Error> for Error {
    fn from(value: Utf8Error) -> Self {
        Self::Utf8(value.to_string())
    }
}

impl From<FromUtf8Error> for Error {
    fn from(value: FromUtf8Error) -> Self {
        Self::Utf8(value.to_string())
    }
}

impl From<crate::backend::Error> for Error {
    fn from(value: crate::backend::Error) -> Self {
        match value {
            crate::backend::Error::ConnectionClosed => Error::ConnectionClosed,
            crate::backend::Error::AlreadyClosed => Error::AlreadyClosed,
            crate::backend::Error::Io(error) => Error::Io(error),
            crate::backend::Error::Tls(error) => Error::Tls(error.to_string()),
            crate::backend::Error::Capacity(error) => Error::Capacity(error.to_string()),
            crate::backend::Error::Protocol(error) => Error::Protocol(error.to_string()),
            crate::backend::Error::WriteBufferFull(message) => {
                Error::WriteBufferFull(Box::new(Message::from(*message)))
            }
            crate::backend::Error::Utf8(error) => Error::Utf8(error),
            crate::backend::Error::AttackAttempt => Error::AttackAttempt,
            crate::backend::Error::Url(error) => Error::Url(error.to_string()),
            #[cfg(feature = "handshake")]
            crate::backend::Error::Http(response) => Error::Http(response),
            #[cfg(feature = "handshake")]
            crate::backend::Error::HttpFormat(error) => Error::HttpFormat(error.to_string()),
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct WebSocket<S> {
    inner: crate::backend::WebSocket<S>,
}

impl<S> WebSocket<S> {
    pub fn from_raw_socket(
        stream: S,
        role: protocol::Role,
        config: Option<protocol::WebSocketConfig>,
    ) -> Self {
        Self {
            inner: crate::backend::WebSocket::from_raw_socket(
                stream,
                role.into(),
                config.map(Into::into),
            ),
        }
    }

    pub fn from_partially_read(
        stream: S,
        part: Vec<u8>,
        role: protocol::Role,
        config: Option<protocol::WebSocketConfig>,
    ) -> Self {
        Self {
            inner: crate::backend::WebSocket::from_partially_read(
                stream,
                part,
                role.into(),
                config.map(Into::into),
            ),
        }
    }

    pub fn into_inner(self) -> S {
        self.inner.into_inner()
    }

    pub fn get_ref(&self) -> &S {
        self.inner.get_ref()
    }

    pub fn get_mut(&mut self) -> &mut S {
        self.inner.get_mut()
    }

    pub fn set_config(&mut self, set_func: impl FnOnce(&mut protocol::WebSocketConfig)) {
        let mut config = self.get_config();
        set_func(&mut config);
        self.inner.set_config(|backend_config| {
            *backend_config = config.into();
        });
    }

    pub fn get_config(&self) -> protocol::WebSocketConfig {
        self.inner.get_config().into()
    }

    pub fn can_read(&self) -> bool {
        self.inner.can_read()
    }

    pub fn can_write(&self) -> bool {
        self.inner.can_write()
    }
}

impl<S> WebSocket<S>
where
    S: Read + Write,
{
    pub fn read(&mut self) -> Result<Message> {
        self.inner.read().map(Message::from).map_err(Error::from)
    }

    #[deprecated(note = "Use `read` instead.")]
    pub fn read_message(&mut self) -> Result<Message> {
        self.read()
    }

    pub fn send(&mut self, message: Message) -> Result<()> {
        self.inner.send(message.into()).map_err(Error::from)
    }

    pub fn write(&mut self, message: Message) -> Result<()> {
        self.inner.write(message.into()).map_err(Error::from)
    }

    #[deprecated(note = "Use `send` instead.")]
    pub fn write_message(&mut self, message: Message) -> Result<()> {
        self.send(message)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.inner.flush().map_err(Error::from)
    }

    #[deprecated(note = "Use `flush` instead.")]
    pub fn write_pending(&mut self) -> Result<()> {
        self.flush()
    }

    pub fn close(&mut self, frame: Option<CloseFrame>) -> Result<()> {
        self.inner.close(frame.map(Into::into)).map_err(Error::from)
    }
}

#[cfg(feature = "handshake")]
pub fn accept<S>(stream: S) -> Result<WebSocket<S>>
where
    S: Read + Write,
{
    accept_with_config(stream, None)
}

#[cfg(feature = "handshake")]
pub fn accept_with_config<S>(
    stream: S,
    config: Option<protocol::WebSocketConfig>,
) -> Result<WebSocket<S>>
where
    S: Read + Write,
{
    crate::backend::accept_with_config(stream, config.map(Into::into))
        .map(|inner| WebSocket { inner })
        .map_err(|error| Error::Other(error.to_string()))
}

#[cfg(feature = "handshake")]
pub fn accept_hdr<S, C>(stream: S, callback: C) -> Result<WebSocket<S>>
where
    S: Read + Write,
    C: handshake::server::Callback,
{
    accept_hdr_with_config(stream, callback, None)
}

#[cfg(feature = "handshake")]
pub fn accept_hdr_with_config<S, C>(
    stream: S,
    callback: C,
    config: Option<protocol::WebSocketConfig>,
) -> Result<WebSocket<S>>
where
    S: Read + Write,
    C: handshake::server::Callback,
{
    let callback = move |request: &handshake::server::Request,
                         response: handshake::server::Response| {
        callback.on_request(request, response)
    };
    crate::backend::accept_hdr_with_config(stream, callback, config.map(Into::into))
        .map(|inner| WebSocket { inner })
        .map_err(|error| Error::Other(error.to_string()))
}

#[cfg(feature = "handshake")]
pub fn client<S, R>(request: R, stream: S) -> Result<(WebSocket<S>, handshake::client::Response)>
where
    S: Read + Write,
    R: client::IntoClientRequest,
{
    client_with_config(request, stream, None)
}

#[cfg(feature = "handshake")]
pub fn client_with_config<S, R>(
    request: R,
    stream: S,
    config: Option<protocol::WebSocketConfig>,
) -> Result<(WebSocket<S>, handshake::client::Response)>
where
    S: Read + Write,
    R: client::IntoClientRequest,
{
    let request = request.into_client_request()?;
    crate::backend::client_with_config(request, stream, config.map(Into::into))
        .map(|(inner, response)| (WebSocket { inner }, response))
        .map_err(|error| Error::Other(error.to_string()))
}

#[cfg(feature = "handshake")]
pub fn connect<R>(
    request: R,
) -> Result<(
    WebSocket<MaybeTlsStream<TcpStream>>,
    handshake::client::Response,
)>
where
    R: client::IntoClientRequest,
{
    connect_with_config(request, None, 3)
}

#[cfg(feature = "handshake")]
pub fn connect_with_config<R>(
    request: R,
    config: Option<protocol::WebSocketConfig>,
    max_redirects: u8,
) -> Result<(
    WebSocket<MaybeTlsStream<TcpStream>>,
    handshake::client::Response,
)>
where
    R: client::IntoClientRequest,
{
    let request = request.into_client_request()?;
    crate::backend::connect_with_config(request, config.map(Into::into), max_redirects)
        .map(|(inner, response)| {
            let config = protocol::WebSocketConfig::from(inner.get_config());
            let stream = MaybeTlsStream::from_backend(inner.into_inner());
            let inner = crate::backend::WebSocket::from_raw_socket(
                stream,
                protocol::Role::Client.into(),
                Some(config.into()),
            );
            (WebSocket { inner }, response)
        })
        .map_err(Error::from)
}

#[cfg(feature = "handshake")]
pub fn uri_mode(uri: &http::Uri) -> Result<Mode> {
    crate::backend::uri_mode(uri)
        .map(Mode::from)
        .map_err(Error::from)
}

#[cfg(feature = "stream")]
#[derive(Debug)]
pub struct WebSocketStream<S>
where
    S: Read + Write,
{
    inner: WebSocket<S>,
}

#[cfg(feature = "stream")]
impl<S> WebSocketStream<S>
where
    S: Read + Write,
{
    fn direct(inner: WebSocket<S>) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> S {
        self.inner.into_inner()
    }

    pub fn get_ref(&self) -> &S {
        self.inner.get_ref()
    }

    pub fn get_mut(&mut self) -> &mut S {
        self.inner.get_mut()
    }

    pub fn get_config(&self) -> protocol::WebSocketConfig {
        self.inner.get_config()
    }

    pub async fn close(&mut self, frame: Option<CloseFrame>) -> Result<()> {
        self.inner.close(frame)
    }

    pub async fn from_raw_socket(
        stream: S,
        role: protocol::Role,
        config: Option<protocol::WebSocketConfig>,
    ) -> Self {
        Self::direct(WebSocket::from_raw_socket(stream, role, config))
    }

    pub async fn from_partially_read(
        stream: S,
        part: Vec<u8>,
        role: protocol::Role,
        config: Option<protocol::WebSocketConfig>,
    ) -> Self {
        Self::direct(WebSocket::from_partially_read(stream, part, role, config))
    }

    pub async fn send(&mut self, message: Message) -> Result<()> {
        self.inner.send(message)
    }
}

#[cfg(feature = "stream")]
impl<S> FuturesStream for WebSocketStream<S>
where
    S: Read + Write + Unpin,
{
    type Item = Result<Message>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(Some(self.inner.read()))
    }
}

#[cfg(feature = "stream")]
impl<S> Sink<Message> for WebSocketStream<S>
where
    S: Read + Write + Unpin,
{
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        self.inner.send(item)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(self.inner.flush())
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(self.inner.close(None))
    }
}

#[cfg(feature = "connect")]
fn request_addr(request: &http::Request<()>) -> Result<String> {
    let uri = request.uri();
    let host = uri
        .host()
        .ok_or_else(|| Error::Other("websocket URI has no host".to_string()))?;
    let port = uri.port_u16().unwrap_or_else(|| {
        if uri.scheme_str() == Some("wss") {
            443
        } else {
            80
        }
    });
    Ok(format!("{host}:{port}"))
}

#[cfg(feature = "connect")]
pub async fn connect_async<R>(
    request: R,
) -> Result<(
    WebSocketStream<MaybeTlsStream<edgerun_tokio::net::TcpStream>>,
    http::Response<Option<Vec<u8>>>,
)>
where
    R: client::IntoClientRequest + Unpin,
{
    connect_async_with_config(request, None, false).await
}

#[cfg(feature = "connect")]
pub async fn connect_async_with_config<R>(
    request: R,
    config: Option<protocol::WebSocketConfig>,
    disable_nagle: bool,
) -> Result<(
    WebSocketStream<MaybeTlsStream<edgerun_tokio::net::TcpStream>>,
    http::Response<Option<Vec<u8>>>,
)>
where
    R: client::IntoClientRequest + Unpin,
{
    let _ = disable_nagle;
    let request = request.into_client_request()?;
    let stream = edgerun_tokio::net::TcpStream::connect(request_addr(&request)?)
        .await
        .map_err(|error| Error::Io(std::io::Error::other(error.to_string())))?;
    let stream = MaybeTlsStream::Plain(stream);
    let (ws, response) = client_with_config(request, stream, config)?;
    Ok((WebSocketStream::direct(ws), response))
}

#[cfg(all(feature = "handshake", feature = "stream"))]
pub async fn client_async<R, S>(
    request: R,
    stream: S,
) -> Result<(WebSocketStream<S>, http::Response<Option<Vec<u8>>>)>
where
    R: client::IntoClientRequest + Unpin,
    S: Read + Write,
{
    client_async_with_config(request, stream, None).await
}

#[cfg(all(feature = "handshake", feature = "stream"))]
pub async fn client_async_with_config<R, S>(
    request: R,
    stream: S,
    config: Option<protocol::WebSocketConfig>,
) -> Result<(WebSocketStream<S>, http::Response<Option<Vec<u8>>>)>
where
    R: client::IntoClientRequest + Unpin,
    S: Read + Write,
{
    client_with_config(request, stream, config)
        .map(|(ws, response)| (WebSocketStream::direct(ws), response))
}

#[cfg(all(feature = "handshake", feature = "stream"))]
pub async fn accept_async<S>(stream: S) -> Result<WebSocketStream<S>>
where
    S: Read + Write,
{
    accept(stream).map(WebSocketStream::direct)
}

#[cfg(all(feature = "handshake", feature = "stream"))]
pub async fn accept_async_with_config<S>(
    stream: S,
    config: Option<protocol::WebSocketConfig>,
) -> Result<WebSocketStream<S>>
where
    S: Read + Write,
{
    accept_with_config(stream, config).map(WebSocketStream::direct)
}

#[cfg(all(feature = "handshake", feature = "stream"))]
pub async fn accept_hdr_async<S, C>(stream: S, callback: C) -> Result<WebSocketStream<S>>
where
    S: Read + Write,
    C: handshake::server::Callback,
{
    accept_hdr_async_with_config(stream, callback, None).await
}

#[cfg(all(feature = "handshake", feature = "stream"))]
pub async fn accept_hdr_async_with_config<S, C>(
    stream: S,
    callback: C,
    config: Option<protocol::WebSocketConfig>,
) -> Result<WebSocketStream<S>>
where
    S: Read + Write,
    C: handshake::server::Callback,
{
    accept_hdr_with_config(stream, callback, config).map(WebSocketStream::direct)
}

mod backend {
    use std::fmt;
    use std::io::{self, BufRead, BufReader, Read, Write};
    use std::net::TcpStream;

    use bytes::Bytes;
    use edgerun_crypto::sha1::Sha1;
    use edgerun_encoding::base64::standard_encode;

    #[cfg(feature = "handshake")]
    pub use edgerun_http as http;

    pub mod extensions {
        #[derive(Copy, Clone, Debug, Default)]
        pub struct ExtensionsConfig {
            pub permessage_deflate: Option<compression::deflate::DeflateConfig>,
        }

        pub mod compression {
            pub mod deflate {
                #[derive(Copy, Clone, Debug, Default)]
                pub struct DeflateConfig;
            }
        }
    }

    pub mod stream {
        use std::io::{Read, Result as IoResult, Write};

        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum Mode {
            Plain,
            Tls,
        }

        #[allow(clippy::large_enum_variant)]
        pub enum MaybeTlsStream<S: Read + Write> {
            Plain(S),
        }

        impl<S: Read + Write> Read for MaybeTlsStream<S> {
            fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
                match self {
                    Self::Plain(stream) => stream.read(buf),
                }
            }
        }

        impl<S: Read + Write> Write for MaybeTlsStream<S> {
            fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
                match self {
                    Self::Plain(stream) => stream.write(buf),
                }
            }

            fn flush(&mut self) -> IoResult<()> {
                match self {
                    Self::Plain(stream) => stream.flush(),
                }
            }
        }
    }

    pub mod protocol {
        pub use self::frame::Frame;
        pub use crate::backend::extensions::ExtensionsConfig;

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Role {
            Server,
            Client,
        }

        pub mod frame {
            use std::fmt;
            use std::io::{Cursor, Read, Write};

            use bytes::Bytes;

            use crate::backend::{Error, Result};

            pub mod coding {
                use std::fmt;

                #[derive(Debug, Eq, PartialEq, Clone, Copy)]
                pub enum CloseCode {
                    Normal,
                    Away,
                    Protocol,
                    Unsupported,
                    Status,
                    Abnormal,
                    Invalid,
                    Policy,
                    Size,
                    Extension,
                    Error,
                    Restart,
                    Again,
                    Tls,
                    Reserved(u16),
                    Iana(u16),
                    Library(u16),
                    Bad(u16),
                }

                impl From<CloseCode> for u16 {
                    fn from(code: CloseCode) -> Self {
                        match code {
                            CloseCode::Normal => 1000,
                            CloseCode::Away => 1001,
                            CloseCode::Protocol => 1002,
                            CloseCode::Unsupported => 1003,
                            CloseCode::Status => 1005,
                            CloseCode::Abnormal => 1006,
                            CloseCode::Invalid => 1007,
                            CloseCode::Policy => 1008,
                            CloseCode::Size => 1009,
                            CloseCode::Extension => 1010,
                            CloseCode::Error => 1011,
                            CloseCode::Restart => 1012,
                            CloseCode::Again => 1013,
                            CloseCode::Tls => 1015,
                            CloseCode::Reserved(code)
                            | CloseCode::Iana(code)
                            | CloseCode::Library(code)
                            | CloseCode::Bad(code) => code,
                        }
                    }
                }

                impl From<&CloseCode> for u16 {
                    fn from(code: &CloseCode) -> Self {
                        (*code).into()
                    }
                }

                impl From<u16> for CloseCode {
                    fn from(code: u16) -> Self {
                        match code {
                            1000 => Self::Normal,
                            1001 => Self::Away,
                            1002 => Self::Protocol,
                            1003 => Self::Unsupported,
                            1005 => Self::Status,
                            1006 => Self::Abnormal,
                            1007 => Self::Invalid,
                            1008 => Self::Policy,
                            1009 => Self::Size,
                            1010 => Self::Extension,
                            1011 => Self::Error,
                            1012 => Self::Restart,
                            1013 => Self::Again,
                            1015 => Self::Tls,
                            1..=999 => Self::Bad(code),
                            1016..=2999 => Self::Reserved(code),
                            3000..=3999 => Self::Iana(code),
                            4000..=4999 => Self::Library(code),
                            _ => Self::Bad(code),
                        }
                    }
                }

                impl fmt::Display for CloseCode {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        write!(f, "{}", u16::from(self))
                    }
                }

                #[derive(Debug, Clone, Copy, Eq, PartialEq)]
                pub enum Data {
                    Continue,
                    Text,
                    Binary,
                    Reserved(u8),
                }

                #[derive(Debug, Clone, Copy, Eq, PartialEq)]
                pub enum Control {
                    Close,
                    Ping,
                    Pong,
                    Reserved(u8),
                }

                #[derive(Debug, Clone, Copy, Eq, PartialEq)]
                pub enum OpCode {
                    Data(Data),
                    Control(Control),
                }

                impl From<OpCode> for u8 {
                    fn from(code: OpCode) -> Self {
                        match code {
                            OpCode::Data(Data::Continue) => 0,
                            OpCode::Data(Data::Text) => 1,
                            OpCode::Data(Data::Binary) => 2,
                            OpCode::Data(Data::Reserved(code)) => code,
                            OpCode::Control(Control::Close) => 8,
                            OpCode::Control(Control::Ping) => 9,
                            OpCode::Control(Control::Pong) => 10,
                            OpCode::Control(Control::Reserved(code)) => code,
                        }
                    }
                }

                impl From<u8> for OpCode {
                    fn from(code: u8) -> Self {
                        match code {
                            0 => Self::Data(Data::Continue),
                            1 => Self::Data(Data::Text),
                            2 => Self::Data(Data::Binary),
                            code @ 3..=7 => Self::Data(Data::Reserved(code)),
                            8 => Self::Control(Control::Close),
                            9 => Self::Control(Control::Ping),
                            10 => Self::Control(Control::Pong),
                            code @ 11..=15 => Self::Control(Control::Reserved(code)),
                            _ => panic!("Bug: OpCode out of range"),
                        }
                    }
                }

                impl fmt::Display for Data {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        write!(f, "{self:?}")
                    }
                }

                impl fmt::Display for Control {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        write!(f, "{self:?}")
                    }
                }

                impl fmt::Display for OpCode {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        match self {
                            Self::Data(data) => data.fmt(f),
                            Self::Control(control) => control.fmt(f),
                        }
                    }
                }
            }

            use coding::{CloseCode, Control, Data, OpCode};

            #[derive(Debug, Clone)]
            pub struct CloseFrame {
                pub code: CloseCode,
                pub reason: crate::Utf8Bytes,
            }

            #[derive(Debug, Clone, Eq, PartialEq)]
            pub struct FrameHeader {
                pub is_final: bool,
                pub rsv1: bool,
                pub rsv2: bool,
                pub rsv3: bool,
                pub opcode: OpCode,
                pub mask: Option<[u8; 4]>,
            }

            impl FrameHeader {
                pub fn parse(cursor: &mut Cursor<impl AsRef<[u8]>>) -> Result<Option<(Self, u64)>> {
                    let bytes = cursor.get_ref().as_ref();
                    let pos = cursor.position() as usize;
                    if bytes.len().saturating_sub(pos) < 2 {
                        return Ok(None);
                    }
                    let first = bytes[pos];
                    let second = bytes[pos + 1];
                    let mut consumed = 2usize;
                    let mut len = u64::from(second & 0x7f);
                    if len == 126 {
                        if bytes.len().saturating_sub(pos) < consumed + 2 {
                            return Ok(None);
                        }
                        len = u64::from(u16::from_be_bytes([
                            bytes[pos + consumed],
                            bytes[pos + consumed + 1],
                        ]));
                        consumed += 2;
                    } else if len == 127 {
                        if bytes.len().saturating_sub(pos) < consumed + 8 {
                            return Ok(None);
                        }
                        let mut len_bytes = [0u8; 8];
                        len_bytes.copy_from_slice(&bytes[pos + consumed..pos + consumed + 8]);
                        len = u64::from_be_bytes(len_bytes);
                        consumed += 8;
                    }
                    let mask = if second & 0x80 != 0 {
                        if bytes.len().saturating_sub(pos) < consumed + 4 {
                            return Ok(None);
                        }
                        let mut mask = [0u8; 4];
                        mask.copy_from_slice(&bytes[pos + consumed..pos + consumed + 4]);
                        consumed += 4;
                        Some(mask)
                    } else {
                        None
                    };
                    cursor.set_position((pos + consumed) as u64);
                    Ok(Some((
                        Self {
                            is_final: first & 0x80 != 0,
                            rsv1: first & 0x40 != 0,
                            rsv2: first & 0x20 != 0,
                            rsv3: first & 0x10 != 0,
                            opcode: OpCode::from(first & 0x0f),
                            mask,
                        },
                        len,
                    )))
                }

                pub fn len(&self, length: u64) -> usize {
                    2 + usize::from(length >= 126) * if length <= 0xffff { 2 } else { 8 }
                        + usize::from(self.mask.is_some()) * 4
                }

                pub fn format(&self, length: u64, output: &mut impl Write) -> Result<()> {
                    let mut first = u8::from(self.opcode);
                    if self.is_final {
                        first |= 0x80;
                    }
                    if self.rsv1 {
                        first |= 0x40;
                    }
                    if self.rsv2 {
                        first |= 0x20;
                    }
                    if self.rsv3 {
                        first |= 0x10;
                    }
                    output.write_all(&[first])?;
                    let mask_bit = if self.mask.is_some() { 0x80 } else { 0 };
                    if length < 126 {
                        output.write_all(&[mask_bit | length as u8])?;
                    } else if length <= 0xffff {
                        output.write_all(&[mask_bit | 126])?;
                        output.write_all(&(length as u16).to_be_bytes())?;
                    } else {
                        output.write_all(&[mask_bit | 127])?;
                        output.write_all(&length.to_be_bytes())?;
                    }
                    if let Some(mask) = self.mask {
                        output.write_all(&mask)?;
                    }
                    Ok(())
                }
            }

            #[derive(Debug, Clone)]
            pub struct Frame {
                pub(crate) header: FrameHeader,
                payload: Bytes,
            }

            impl Frame {
                pub fn message(data: Vec<u8>, opcode: OpCode, is_final: bool) -> Self {
                    Self {
                        header: FrameHeader {
                            is_final,
                            rsv1: false,
                            rsv2: false,
                            rsv3: false,
                            opcode,
                            mask: None,
                        },
                        payload: data.into(),
                    }
                }

                pub fn pong(data: Vec<u8>) -> Self {
                    Self::message(data, OpCode::Control(Control::Pong), true)
                }

                pub fn ping(data: Vec<u8>) -> Self {
                    Self::message(data, OpCode::Control(Control::Ping), true)
                }

                pub fn close(message: Option<CloseFrame>) -> Self {
                    let mut payload = Vec::new();
                    if let Some(message) = message {
                        payload.extend_from_slice(&u16::from(message.code).to_be_bytes());
                        payload.extend_from_slice(message.reason.as_bytes());
                    }
                    Self::message(payload, OpCode::Control(Control::Close), true)
                }

                pub fn from_payload(header: FrameHeader, payload: Bytes) -> Self {
                    Self { header, payload }
                }

                pub fn format(self, output: &mut impl Write) -> Result<()> {
                    self.header.format(self.payload.len() as u64, output)?;
                    let mut payload = self.payload.to_vec();
                    if let Some(mask) = self.header.mask {
                        for (index, byte) in payload.iter_mut().enumerate() {
                            *byte ^= mask[index % 4];
                        }
                    }
                    output.write_all(&payload)?;
                    Ok(())
                }

                pub fn header(&self) -> &FrameHeader {
                    &self.header
                }

                pub fn len(&self) -> usize {
                    self.payload.len()
                }

                pub fn is_empty(&self) -> bool {
                    self.payload.is_empty()
                }

                pub fn payload(&self) -> &[u8] {
                    &self.payload
                }

                pub fn into_payload(self) -> Bytes {
                    self.payload
                }

                pub(crate) fn read_from(reader: &mut impl Read) -> Result<Self> {
                    let mut first_two = [0u8; 2];
                    reader.read_exact(&mut first_two)?;
                    let mut header_bytes = first_two.to_vec();
                    let mut len = u64::from(first_two[1] & 0x7f);
                    if len == 126 {
                        let mut bytes = [0u8; 2];
                        reader.read_exact(&mut bytes)?;
                        header_bytes.extend_from_slice(&bytes);
                        len = u64::from(u16::from_be_bytes(bytes));
                    } else if len == 127 {
                        let mut bytes = [0u8; 8];
                        reader.read_exact(&mut bytes)?;
                        header_bytes.extend_from_slice(&bytes);
                        len = u64::from_be_bytes(bytes);
                    }
                    if first_two[1] & 0x80 != 0 {
                        let mut mask = [0u8; 4];
                        reader.read_exact(&mut mask)?;
                        header_bytes.extend_from_slice(&mask);
                    }
                    let (header, _) = FrameHeader::parse(&mut Cursor::new(header_bytes))?
                        .ok_or_else(|| Error::Protocol("incomplete frame header".to_string()))?;
                    let mut payload = vec![0u8; len as usize];
                    reader.read_exact(&mut payload)?;
                    if let Some(mask) = header.mask {
                        for (index, byte) in payload.iter_mut().enumerate() {
                            *byte ^= mask[index % 4];
                        }
                    }
                    Ok(Self {
                        header: FrameHeader {
                            mask: None,
                            ..header
                        },
                        payload: payload.into(),
                    })
                }
            }

            impl fmt::Display for Frame {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(
                        f,
                        "{:?} frame, {} bytes",
                        self.header.opcode,
                        self.payload.len()
                    )
                }
            }

            pub(crate) fn opcode_message(payload: Bytes, opcode: OpCode) -> super::super::Message {
                match opcode {
                    OpCode::Data(Data::Text) => super::super::Message::Text(
                        String::from_utf8_lossy(&payload).to_string().into(),
                    ),
                    OpCode::Data(Data::Binary) => super::super::Message::Binary(payload),
                    OpCode::Control(Control::Ping) => super::super::Message::Ping(payload),
                    OpCode::Control(Control::Pong) => super::super::Message::Pong(payload),
                    OpCode::Control(Control::Close) => {
                        let close = if payload.len() >= 2 {
                            let code =
                                CloseCode::from(u16::from_be_bytes([payload[0], payload[1]]));
                            let reason = String::from_utf8_lossy(&payload[2..]).to_string().into();
                            Some(CloseFrame { code, reason })
                        } else {
                            None
                        };
                        super::super::Message::Close(close)
                    }
                    _ => super::super::Message::Frame(Frame::from_payload(
                        FrameHeader {
                            is_final: true,
                            rsv1: false,
                            rsv2: false,
                            rsv3: false,
                            opcode,
                            mask: None,
                        },
                        payload,
                    )),
                }
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct WebSocketConfig {
            pub read_buffer_size: usize,
            pub write_buffer_size: usize,
            pub max_write_buffer_size: usize,
            pub max_message_size: Option<usize>,
            pub max_frame_size: Option<usize>,
            pub accept_unmasked_frames: bool,
            pub extensions: ExtensionsConfig,
        }

        impl Default for WebSocketConfig {
            fn default() -> Self {
                Self {
                    read_buffer_size: 128 * 1024,
                    write_buffer_size: 128 * 1024,
                    max_write_buffer_size: usize::MAX,
                    max_message_size: Some(64 << 20),
                    max_frame_size: Some(16 << 20),
                    accept_unmasked_frames: false,
                    extensions: ExtensionsConfig::default(),
                }
            }
        }

        impl WebSocketConfig {
            pub fn read_buffer_size(mut self, value: usize) -> Self {
                self.read_buffer_size = value;
                self
            }

            pub fn write_buffer_size(mut self, value: usize) -> Self {
                self.write_buffer_size = value;
                self
            }

            pub fn max_write_buffer_size(mut self, value: usize) -> Self {
                self.max_write_buffer_size = value;
                self
            }

            pub fn max_message_size(mut self, value: Option<usize>) -> Self {
                self.max_message_size = value;
                self
            }

            pub fn max_frame_size(mut self, value: Option<usize>) -> Self {
                self.max_frame_size = value;
                self
            }

            pub fn accept_unmasked_frames(mut self, value: bool) -> Self {
                self.accept_unmasked_frames = value;
                self
            }
        }

        pub use frame::CloseFrame;
        pub use frame::FrameHeader;
        pub use frame::coding::CloseCode;
    }

    pub use protocol::WebSocketConfig;
    pub use protocol::frame::CloseFrame;
    pub use protocol::frame::Frame;
    pub use protocol::frame::coding::CloseCode;

    #[derive(Debug, Clone)]
    pub enum Message {
        Text(crate::Utf8Bytes),
        Binary(Bytes),
        Ping(Bytes),
        Pong(Bytes),
        Close(Option<CloseFrame>),
        Frame(Frame),
    }

    #[derive(Debug)]
    pub enum Error {
        ConnectionClosed,
        AlreadyClosed,
        Io(io::Error),
        Tls(String),
        Capacity(String),
        Protocol(String),
        WriteBufferFull(Box<Message>),
        Utf8(String),
        AttackAttempt,
        Url(String),
        #[cfg(feature = "handshake")]
        Http(Box<http::Response<Option<Vec<u8>>>>),
        #[cfg(feature = "handshake")]
        HttpFormat(String),
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::ConnectionClosed => f.write_str("connection closed"),
                Self::AlreadyClosed => f.write_str("connection already closed"),
                Self::Io(error) => write!(f, "{error}"),
                Self::Tls(error)
                | Self::Capacity(error)
                | Self::Protocol(error)
                | Self::Utf8(error)
                | Self::Url(error) => f.write_str(error),
                Self::WriteBufferFull(_) => f.write_str("write buffer is full"),
                Self::AttackAttempt => f.write_str("attack attempt detected"),
                #[cfg(feature = "handshake")]
                Self::Http(response) => write!(f, "HTTP {}", response.status()),
                #[cfg(feature = "handshake")]
                Self::HttpFormat(error) => f.write_str(error),
            }
        }
    }

    impl std::error::Error for Error {}

    impl From<io::Error> for Error {
        fn from(error: io::Error) -> Self {
            Self::Io(error)
        }
    }

    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Debug)]
    pub struct WebSocket<S> {
        stream: S,
        role: protocol::Role,
        config: WebSocketConfig,
        closed: bool,
    }

    impl<S> WebSocket<S> {
        pub fn from_raw_socket(
            stream: S,
            role: protocol::Role,
            config: Option<WebSocketConfig>,
        ) -> Self {
            Self {
                stream,
                role,
                config: config.unwrap_or_default(),
                closed: false,
            }
        }

        pub fn from_partially_read(
            stream: S,
            _part: Vec<u8>,
            role: protocol::Role,
            config: Option<WebSocketConfig>,
        ) -> Self {
            Self::from_raw_socket(stream, role, config)
        }

        pub fn into_inner(self) -> S {
            self.stream
        }

        pub fn get_ref(&self) -> &S {
            &self.stream
        }

        pub fn get_mut(&mut self) -> &mut S {
            &mut self.stream
        }

        pub fn set_config(&mut self, set_func: impl FnOnce(&mut WebSocketConfig)) {
            set_func(&mut self.config);
        }

        pub fn get_config(&self) -> &WebSocketConfig {
            &self.config
        }

        pub fn can_read(&self) -> bool {
            !self.closed
        }

        pub fn can_write(&self) -> bool {
            !self.closed
        }
    }

    impl<S: Read + Write> WebSocket<S> {
        pub fn read(&mut self) -> Result<Message> {
            if self.closed {
                return Err(Error::AlreadyClosed);
            }
            let frame = Frame::read_from(&mut self.stream)?;
            if let Some(max) = self.config.max_frame_size {
                if frame.len() > max {
                    return Err(Error::Capacity("frame exceeds max frame size".to_string()));
                }
            }
            let opcode = frame.header().opcode;
            let message = protocol::frame::opcode_message(frame.into_payload(), opcode);
            if matches!(message, Message::Close(_)) {
                self.closed = true;
            }
            Ok(message)
        }

        pub fn send(&mut self, message: Message) -> Result<()> {
            self.write(message)?;
            self.flush()
        }

        pub fn write(&mut self, message: Message) -> Result<()> {
            if self.closed {
                return Err(Error::AlreadyClosed);
            }
            let mut frame = match message {
                Message::Text(text) => Frame::message(
                    String::from(text).into_bytes(),
                    protocol::frame::coding::OpCode::Data(protocol::frame::coding::Data::Text),
                    true,
                ),
                Message::Binary(bytes) => Frame::message(
                    bytes.to_vec(),
                    protocol::frame::coding::OpCode::Data(protocol::frame::coding::Data::Binary),
                    true,
                ),
                Message::Ping(bytes) => Frame::ping(bytes.to_vec()),
                Message::Pong(bytes) => Frame::pong(bytes.to_vec()),
                Message::Close(frame) => {
                    self.closed = true;
                    Frame::close(frame)
                }
                Message::Frame(frame) => frame,
            };
            if matches!(self.role, protocol::Role::Client) {
                let mut mask = [0u8; 4];
                edgerun_crypto::fill_random(&mut mask)
                    .expect("edgerun secure random source unavailable");
                frame.header.mask = Some(mask);
            }
            frame.format(&mut self.stream)
        }

        pub fn flush(&mut self) -> Result<()> {
            self.stream.flush().map_err(Error::from)
        }

        pub fn close(&mut self, frame: Option<CloseFrame>) -> Result<()> {
            self.write(Message::Close(frame))?;
            self.flush()
        }
    }

    #[cfg(feature = "handshake")]
    pub mod handshake {
        pub mod server {
            use std::io::Write;

            use edgerun_encoding::base64::standard_encode;

            use crate::backend::{Error, Result, http};

            pub fn create_response(request: &http::Request<()>) -> Result<http::Response<()>> {
                create_response_with_body(request, || ())
            }

            pub fn create_response_with_body<T1, T2>(
                request: &http::Request<T1>,
                generate_body: impl FnOnce() -> T2,
            ) -> Result<http::Response<T2>> {
                let key = request
                    .headers()
                    .get("sec-websocket-key")
                    .ok_or_else(|| Error::Protocol("missing Sec-WebSocket-Key".to_string()))?
                    .to_str()
                    .map_err(|error| Error::HttpFormat(error.to_string()))?;
                http::Response::<()>::builder()
                    .status(http::StatusCode::SWITCHING_PROTOCOLS)
                    .header("Connection", "Upgrade")
                    .header("Upgrade", "websocket")
                    .header("Sec-WebSocket-Accept", websocket_accept(key))
                    .body(generate_body())
                    .map_err(|error| Error::HttpFormat(error.to_string()))
            }

            pub fn write_response<T>(
                mut writer: impl Write,
                response: &http::Response<T>,
            ) -> Result<()> {
                let reason = response.status().canonical_reason().unwrap_or("");
                write!(
                    writer,
                    "HTTP/1.1 {} {}\r\n",
                    response.status().as_u16(),
                    reason
                )?;
                for (name, value) in response.headers() {
                    write!(
                        writer,
                        "{}: {}\r\n",
                        name.as_str(),
                        value.to_str().unwrap_or("")
                    )?;
                }
                writer.write_all(b"\r\n")?;
                Ok(())
            }

            fn websocket_accept(key: &str) -> String {
                let mut data = key.as_bytes().to_vec();
                data.extend_from_slice(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
                standard_encode(&edgerun_crypto::sha1::Sha1::digest(data))
            }
        }
    }

    #[cfg(feature = "handshake")]
    fn read_http_request(stream: &mut impl Read) -> Result<http::Request<()>> {
        let mut reader = BufReader::new(stream);
        let mut first = String::new();
        reader.read_line(&mut first)?;
        let mut parts = first.split_whitespace();
        let method = parts.next().unwrap_or("GET");
        let target = parts.next().unwrap_or("/");
        let mut builder = http::Request::<()>::builder()
            .method(method)
            .uri(format!("ws://localhost{target}"));
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                builder = builder.header(name.trim(), value.trim());
            }
        }
        builder
            .body(())
            .map_err(|error| Error::HttpFormat(error.to_string()))
    }

    #[cfg(feature = "handshake")]
    fn read_http_response(stream: &mut impl Read) -> Result<http::Response<Option<Vec<u8>>>> {
        let mut reader = BufReader::new(stream);
        let mut first = String::new();
        reader.read_line(&mut first)?;
        let status = first
            .split_whitespace()
            .nth(1)
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| Error::HttpFormat("invalid HTTP response status".to_string()))?;
        let mut builder = http::Response::<Option<Vec<u8>>>::builder().status(
            http::StatusCode::from_u16(status)
                .map_err(|error| Error::HttpFormat(error.to_string()))?,
        );
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                builder = builder.header(name.trim(), value.trim());
            }
        }
        builder
            .body(None)
            .map_err(|error| Error::HttpFormat(error.to_string()))
    }

    #[cfg(feature = "handshake")]
    pub fn accept_with_config<S: Read + Write>(
        mut stream: S,
        config: Option<WebSocketConfig>,
    ) -> Result<WebSocket<S>> {
        let request = read_http_request(&mut stream)?;
        let response = handshake::server::create_response(&request)?;
        handshake::server::write_response(&mut stream, &response)?;
        Ok(WebSocket::from_raw_socket(
            stream,
            protocol::Role::Server,
            config,
        ))
    }

    #[cfg(feature = "handshake")]
    pub fn accept_hdr_with_config<S, C>(
        mut stream: S,
        callback: C,
        config: Option<WebSocketConfig>,
    ) -> Result<WebSocket<S>>
    where
        S: Read + Write,
        C: FnOnce(
            &http::Request<()>,
            http::Response<()>,
        )
            -> std::result::Result<http::Response<()>, http::Response<Option<String>>>,
    {
        let request = read_http_request(&mut stream)?;
        let response = handshake::server::create_response(&request)?;
        let response = callback(&request, response).map_err(|response| {
            Error::Http(Box::new(
                http::Response::<Option<Vec<u8>>>::builder()
                    .status(response.status())
                    .body(response.into_body().map(|body| body.into_bytes()))
                    .expect("response"),
            ))
        })?;
        handshake::server::write_response(&mut stream, &response)?;
        Ok(WebSocket::from_raw_socket(
            stream,
            protocol::Role::Server,
            config,
        ))
    }

    #[cfg(feature = "handshake")]
    pub mod client {
        use std::io::Write;

        use crate::backend::{
            Error, Result, WebSocket, WebSocketConfig, http, protocol, read_http_response,
        };

        pub fn client_with_config<S: std::io::Read + std::io::Write>(
            request: http::Request<()>,
            mut stream: S,
            config: Option<WebSocketConfig>,
        ) -> Result<(WebSocket<S>, http::Response<Option<Vec<u8>>>)> {
            let path = request.uri().path_and_query();
            write!(stream, "GET {path} HTTP/1.1\r\n")?;
            for (name, value) in request.headers() {
                write!(
                    stream,
                    "{}: {}\r\n",
                    name.as_str(),
                    value.to_str().unwrap_or("")
                )?;
            }
            stream.write_all(b"\r\n")?;
            stream.flush()?;
            let response = read_http_response(&mut stream)?;
            if response.status() != http::StatusCode::SWITCHING_PROTOCOLS {
                return Err(Error::Http(Box::new(response)));
            }
            Ok((
                WebSocket::from_raw_socket(stream, protocol::Role::Client, config),
                response,
            ))
        }

        pub fn uri_mode(uri: &http::Uri) -> Result<super::stream::Mode> {
            match uri.scheme_str() {
                Some("ws") => Ok(super::stream::Mode::Plain),
                Some("wss") => Ok(super::stream::Mode::Tls),
                _ => Err(Error::Url("unsupported websocket URI scheme".to_string())),
            }
        }

        pub fn connect_with_config(
            request: http::Request<()>,
            config: Option<WebSocketConfig>,
            _max_redirects: u8,
        ) -> Result<(
            WebSocket<super::stream::MaybeTlsStream<std::net::TcpStream>>,
            http::Response<Option<Vec<u8>>>,
        )> {
            let host = request
                .uri()
                .host()
                .ok_or_else(|| Error::Url("websocket URI has no host".to_string()))?;
            let port = request.uri().port_u16().unwrap_or_else(|| {
                if request.uri().scheme_str() == Some("wss") {
                    443
                } else {
                    80
                }
            });
            if request.uri().scheme_str() == Some("wss") {
                return Err(Error::Tls(
                    "TLS websocket connections require an EdgeRun TLS connector".to_string(),
                ));
            }
            let stream = std::net::TcpStream::connect((host, port))?;
            client_with_config(
                request,
                super::stream::MaybeTlsStream::Plain(stream),
                config,
            )
        }
    }

    #[cfg(feature = "handshake")]
    pub use client::client_with_config;
    #[cfg(feature = "handshake")]
    pub use client::connect_with_config;
    #[cfg(feature = "handshake")]
    pub use client::uri_mode;

    #[cfg(feature = "handshake")]
    pub fn connect(
        request: http::Request<()>,
    ) -> Result<(
        WebSocket<stream::MaybeTlsStream<TcpStream>>,
        http::Response<Option<Vec<u8>>>,
    )> {
        client::connect_with_config(request, None, 3)
    }
}

#[cfg(test)]
mod tests {
    use super::MaybeTlsStream;
    use super::MaybeTlsStreamExt;
    use super::Message;
    use super::Mode;
    use super::WebSocket;
    use super::client::ClientRequestBuilder;
    use super::client::IntoClientRequest;
    use super::protocol::Role;
    use super::protocol::WebSocketConfig;
    use super::protocol::frame::Frame;
    use super::protocol::frame::FrameHeader;
    use super::protocol::frame::coding::Data;
    use super::protocol::frame::coding::OpCode;
    use bytes::Bytes;
    use bytes::BytesMut;
    use std::borrow::Borrow;
    use std::io::Cursor;
    use std::net::TcpStream;

    #[test]
    fn string_uri_builds_websocket_request() {
        let request = "wss://example.com/realtime?model=test"
            .into_client_request()
            .expect("request");

        assert_eq!(request.method(), "GET");
        assert_eq!(
            request.uri().to_string(),
            "wss://example.com/realtime?model=test"
        );
        assert_eq!(request.headers()["Host"], "example.com");
        assert_eq!(request.headers()["Connection"], "Upgrade");
        assert_eq!(request.headers()["Upgrade"], "websocket");
        assert_eq!(request.headers()["Sec-WebSocket-Version"], "13");
        assert_eq!(request.headers()["Sec-WebSocket-Key"].as_bytes().len(), 24);
    }

    #[test]
    fn client_request_builder_adds_headers_and_subprotocols() {
        let uri = "ws://localhost/socket".parse().expect("uri");
        let request = ClientRequestBuilder::new(uri)
            .with_header("Authorization", "Bearer test")
            .with_sub_protocol("realtime")
            .with_sub_protocol("json")
            .into_client_request()
            .expect("request");

        assert_eq!(request.headers()["Authorization"], "Bearer test");
        assert_eq!(
            request.headers()["Sec-WebSocket-Protocol"],
            "realtime, json"
        );
    }

    #[test]
    fn client_module_exposes_upstream_compatibility_items() {
        let uri: super::http::Uri = "wss://example.com/socket".parse().expect("uri");
        let request: super::client::Request = uri.into_client_request().expect("request");
        let mode = super::client::uri_mode(request.uri()).expect("mode");
        let key = super::handshake::client::generate_key();

        assert_eq!(mode, Mode::Tls);
        assert_eq!(key.len(), 24);
    }

    #[test]
    fn server_handshake_helpers_create_and_write_response() {
        let request = "ws://localhost/socket"
            .into_client_request()
            .expect("request");
        let response = super::handshake::server::create_response(&request).expect("response");
        let mut bytes = Vec::new();

        super::handshake::server::write_response(&mut bytes, &response).expect("write");

        let response = String::from_utf8(bytes).expect("utf8");
        assert!(response.starts_with("HTTP/1.1 101 Switching Protocols\r\n"));
        assert!(response.contains("sec-websocket-accept: "));
    }

    #[test]
    fn message_helpers_match_expected_shapes() {
        let text = Message::from("hello");
        assert!(text.is_text());
        assert_eq!(text.len(), 5);
        assert_eq!(text.to_text().expect("text"), "hello");

        let binary = Message::from(Bytes::from_static(b"world"));
        assert!(binary.is_binary());
        assert_eq!(binary.to_text().expect("utf8"), "world");
        assert_eq!(binary.into_data(), Bytes::from_static(b"world"));

        let close = Message::Close(None);
        assert!(close.is_close());
        assert!(close.is_empty());
        assert_eq!(close.into_text().expect("close text").as_str(), "");
    }

    #[test]
    fn utf8_and_close_frame_compatibility_traits_work() {
        let text = super::Utf8Bytes::try_from(BytesMut::from(&b"hello"[..])).expect("utf8");
        let borrowed: &str = text.borrow();
        let from_vec = super::Utf8Bytes::try_from(b"world".to_vec()).expect("utf8");
        let close = super::CloseFrame {
            code: super::CloseCode::Normal,
            reason: from_vec,
        };

        assert_eq!(borrowed, "hello");
        assert_eq!(close.to_string(), "world (1000)");
    }

    #[test]
    fn frame_constructors_and_header_format_match_expected_shapes() {
        let header = FrameHeader {
            opcode: OpCode::Data(Data::Text),
            ..FrameHeader::default()
        };
        let mut header_bytes = Vec::new();
        header.format(5, &mut header_bytes).expect("format header");
        let parsed = FrameHeader::parse(&mut Cursor::new(header_bytes.clone()))
            .expect("parse")
            .expect("complete");

        let mut frame_bytes = Vec::new();
        let frame = Frame::from_payload(header, Bytes::from_static(b"hello"));
        frame.format(&mut frame_bytes).expect("format frame");

        assert_eq!(header_bytes, &[0x81, 0x05]);
        assert_eq!(parsed.0.opcode, OpCode::Data(Data::Text));
        assert_eq!(parsed.1, 5);
        assert_eq!(frame_bytes, b"\x81\x05hello");
        assert_eq!(
            Frame::message(Bytes::from_static(b"hi"), OpCode::Data(Data::Text), true)
                .header()
                .opcode,
            OpCode::Data(Data::Text)
        );
        assert_eq!(Frame::ping(Bytes::from_static(b"hi")).payload(), b"hi");
        assert_eq!(Frame::pong(Bytes::from_static(b"ok")).payload(), b"ok");
        assert!(Frame::close(None).payload().is_empty());
    }

    #[test]
    fn websocket_config_builder_methods_update_fields() {
        let config = WebSocketConfig::default()
            .read_buffer_size(8)
            .write_buffer_size(16)
            .max_write_buffer_size(32)
            .max_message_size(Some(64))
            .max_frame_size(Some(128))
            .accept_unmasked_frames(true);

        assert_eq!(config.read_buffer_size, 8);
        assert_eq!(config.write_buffer_size, 16);
        assert_eq!(config.max_write_buffer_size, 32);
        assert_eq!(config.max_message_size, Some(64));
        assert_eq!(config.max_frame_size, Some(128));
        assert!(config.accept_unmasked_frames);
    }

    #[test]
    fn uri_mode_identifies_plain_and_tls_websockets() {
        assert_eq!(
            super::uri_mode(&"ws://example.com/socket".parse().expect("uri")).expect("mode"),
            Mode::Plain
        );
        assert_eq!(
            super::uri_mode(&"wss://example.com/socket".parse().expect("uri")).expect("mode"),
            Mode::Tls
        );
        assert!(super::uri_mode(&"http://example.com".parse().expect("uri")).is_err());
    }

    #[test]
    fn maybe_tls_stream_ext_exposes_plain_streams() {
        let mut stream = MaybeTlsStream::Plain(Cursor::new(vec![1, 2, 3]));

        assert!(stream.is_plain());
        assert_eq!(stream.plain_ref().expect("plain").get_ref(), &[1, 2, 3]);
        stream
            .plain_mut()
            .expect("plain")
            .get_mut()
            .extend_from_slice(&[4, 5]);
        assert_eq!(
            stream.plain_ref().expect("plain").get_ref(),
            &[1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn websocket_wrapper_exposes_stream_and_config() {
        let stream = Cursor::new(Vec::<u8>::new());
        let mut websocket = WebSocket::from_raw_socket(
            stream,
            Role::Client,
            Some(WebSocketConfig::default().read_buffer_size(1024)),
        );

        assert!(websocket.can_read());
        assert!(websocket.can_write());
        assert_eq!(websocket.get_config().read_buffer_size, 1024);

        websocket.set_config(|config| {
            config.write_buffer_size = 2048;
        });

        assert_eq!(websocket.get_config().write_buffer_size, 2048);
        websocket.get_mut().get_mut().extend_from_slice(b"owned");
        assert_eq!(websocket.into_inner().into_inner(), b"owned");
    }

    #[test]
    fn connected_websocket_type_remains_send() {
        fn assert_send<T: Send>() {}

        assert_send::<WebSocket<MaybeTlsStream<TcpStream>>>();
    }
}
