//! Compatibility surface for crates that currently use `tungstenite`.
//!
//! This crate owns the public protocol types used by Edgerun Codex. The current
//! backend adapter still converts to the upstream fork internally while we fill
//! in the Edgerun WebSocket implementation behind this boundary.

use bytes::Bytes;
use std::fmt;
use std::io;
use std::ops::Deref;
use std::str::Utf8Error;
use std::string::FromUtf8Error;

pub mod client {
    use crate::Error;
    use crate::Result;
    use edgerun_encoding::base64::standard_encode;
    use http::HeaderName;
    use http::Request;
    use http::Uri;

    pub type ClientRequest = Request<()>;

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

            Request::builder()
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
        bytes[..8].copy_from_slice(&edgerun_random::u64().to_be_bytes());
        bytes[8..].copy_from_slice(&edgerun_random::u64().to_be_bytes());
        standard_encode(&bytes)
    }
}

pub use client::ClientRequestBuilder;

pub mod handshake {
    pub mod server {
        pub type Request = http::Request<()>;
        pub type Response = http::Response<()>;
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

    pub mod frame {
        pub use crate::CloseFrame;
        pub use crate::Frame;
        pub use crate::Utf8Bytes;

        pub mod coding {
            pub use crate::CloseCode;
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
}

impl From<extensions::ExtensionsConfig> for crate::backend::extensions::ExtensionsConfig {
    fn from(value: extensions::ExtensionsConfig) -> Self {
        let mut config = crate::backend::extensions::ExtensionsConfig::default();
        config.permessage_deflate = value
            .permessage_deflate
            .map(|_| crate::backend::extensions::compression::deflate::DeflateConfig::default());
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

#[derive(Debug, Clone)]
pub struct Frame {
    inner: crate::backend::Frame,
}

impl Frame {
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
        self.inner.into_payload()
    }

    pub fn into_text(self) -> Result<Utf8Bytes> {
        self.into_payload().try_into().map_err(Error::from)
    }

    pub fn to_text(&self) -> Result<&str> {
        std::str::from_utf8(self.payload()).map_err(Error::from)
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
            Message::Binary(bytes) => crate::backend::Message::Binary(bytes),
            Message::Ping(bytes) => crate::backend::Message::Ping(bytes),
            Message::Pong(bytes) => crate::backend::Message::Pong(bytes),
            Message::Close(frame) => crate::backend::Message::Close(frame.map(Into::into)),
            Message::Frame(frame) => crate::backend::Message::Frame(frame.inner),
        }
    }
}

impl From<crate::backend::Message> for Message {
    fn from(value: crate::backend::Message) -> Self {
        match value {
            crate::backend::Message::Text(text) => Message::Text(text.to_string().into()),
            crate::backend::Message::Binary(bytes) => Message::Binary(bytes),
            crate::backend::Message::Ping(bytes) => Message::Ping(bytes),
            crate::backend::Message::Pong(bytes) => Message::Pong(bytes),
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
    Http(Box<http::Response<Option<Vec<u8>>>>),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ConnectionClosed => f.write_str("Connection closed normally"),
            Error::AlreadyClosed => f.write_str("Trying to work with closed connection"),
            Error::Io(error) => write!(f, "IO error: {error}"),
            Error::Http(response) => write!(f, "HTTP error: {}", response.status()),
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
        Self::Other(value.to_string())
    }
}

impl From<FromUtf8Error> for Error {
    fn from(value: FromUtf8Error) -> Self {
        Self::Other(value.to_string())
    }
}

impl From<crate::backend::Error> for Error {
    fn from(value: crate::backend::Error) -> Self {
        match value {
            crate::backend::Error::ConnectionClosed => Error::ConnectionClosed,
            crate::backend::Error::AlreadyClosed => Error::AlreadyClosed,
            crate::backend::Error::Io(error) => Error::Io(error),
            crate::backend::Error::Http(response) => Error::Http(response),
            other => Error::Other(other.to_string()),
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[doc(hidden)]
pub mod backend {
    pub use tungstenite::Error;
    pub use tungstenite::Message;
    pub use tungstenite::extensions;
    pub use tungstenite::protocol;
    pub use tungstenite::protocol::CloseFrame;
    pub use tungstenite::protocol::WebSocketConfig;
    pub use tungstenite::protocol::frame::Frame;
    pub use tungstenite::protocol::frame::coding::CloseCode;
}

#[cfg(test)]
mod tests {
    use super::Message;
    use super::client::ClientRequestBuilder;
    use super::client::IntoClientRequest;
    use super::protocol::WebSocketConfig;
    use bytes::Bytes;

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
}
