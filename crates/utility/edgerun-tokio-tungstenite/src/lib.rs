//! Edgerun-native async facade for tungstenite-style WebSocket APIs.

use std::io::{Read, Write};
use std::pin::Pin;
use std::task::{Context, Poll};

use edgerun_futures::Sink;
use edgerun_futures::Stream;
use edgerun_tokio::net::TcpStream;
use edgerun_tungstenite::client::IntoClientRequest;
use edgerun_tungstenite::protocol::CloseFrame;
use edgerun_tungstenite::protocol::Role;
use edgerun_tungstenite::protocol::WebSocketConfig;

pub mod tungstenite {
    pub use edgerun_tungstenite::*;
}

pub use edgerun_tungstenite::Error;
pub use edgerun_tungstenite::Message;
#[cfg(feature = "handshake")]
pub use edgerun_tungstenite::http;

#[non_exhaustive]
#[derive(Clone)]
pub enum Connector {
    Plain,
}

#[cfg(feature = "stream")]
#[derive(Debug)]
pub struct MaybeTlsStream<S>(edgerun_tungstenite::MaybeTlsStream<S>)
where
    S: Read + Write;

#[cfg(feature = "stream")]
impl<S> MaybeTlsStream<S>
where
    S: Read + Write,
{
    fn plain(stream: S) -> Self {
        Self(edgerun_tungstenite::MaybeTlsStream::Plain(stream))
    }

    pub fn get_ref(&self) -> &S {
        match &self.0 {
            edgerun_tungstenite::MaybeTlsStream::Plain(stream) => stream,
            _ => panic!("TLS stream does not expose an Edgerun plain stream reference"),
        }
    }

    pub fn get_mut(&mut self) -> &mut S {
        match &mut self.0 {
            edgerun_tungstenite::MaybeTlsStream::Plain(stream) => stream,
            _ => panic!("TLS stream does not expose an Edgerun plain stream reference"),
        }
    }
}

#[cfg(feature = "stream")]
impl<S> Read for MaybeTlsStream<S>
where
    S: Read + Write,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

#[cfg(feature = "stream")]
impl<S> Write for MaybeTlsStream<S>
where
    S: Read + Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

#[cfg(feature = "stream")]
pub trait MaybeTlsStreamExt<S>
where
    S: Read + Write,
{
    fn is_plain(&self) -> bool;
    fn plain_ref(&self) -> Option<&S>;
    fn plain_mut(&mut self) -> Option<&mut S>;
}

#[cfg(feature = "stream")]
impl<S> MaybeTlsStreamExt<S> for MaybeTlsStream<S>
where
    S: Read + Write,
{
    fn is_plain(&self) -> bool {
        matches!(self.0, edgerun_tungstenite::MaybeTlsStream::Plain(_))
    }

    fn plain_ref(&self) -> Option<&S> {
        match &self.0 {
            edgerun_tungstenite::MaybeTlsStream::Plain(stream) => Some(stream),
            _ => None,
        }
    }

    fn plain_mut(&mut self) -> Option<&mut S> {
        match &mut self.0 {
            edgerun_tungstenite::MaybeTlsStream::Plain(stream) => Some(stream),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct WebSocketStream<S>
where
    S: Read + Write,
{
    inner: edgerun_tungstenite::WebSocket<S>,
}

impl<S> WebSocketStream<S>
where
    S: Read + Write,
{
    fn direct(inner: edgerun_tungstenite::WebSocket<S>) -> Self {
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

    pub fn get_config(&self) -> WebSocketConfig {
        self.inner.get_config()
    }

    pub async fn close(&mut self, frame: Option<CloseFrame>) -> Result<(), Error> {
        self.inner.close(frame)
    }

    pub async fn from_raw_socket(stream: S, role: Role, config: Option<WebSocketConfig>) -> Self {
        Self::direct(edgerun_tungstenite::WebSocket::from_raw_socket(
            stream, role, config,
        ))
    }

    pub async fn from_partially_read(
        stream: S,
        part: Vec<u8>,
        role: Role,
        config: Option<WebSocketConfig>,
    ) -> Self {
        Self::direct(edgerun_tungstenite::WebSocket::from_partially_read(
            stream, part, role, config,
        ))
    }

    pub async fn send(&mut self, message: Message) -> Result<(), Error> {
        self.inner.send(message)
    }
}

impl<S> Stream for WebSocketStream<S>
where
    S: Read + Write + Unpin,
{
    type Item = Result<Message, Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(Some(self.inner.read()))
    }
}

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
fn request_addr(request: &http::Request<()>) -> Result<String, Error> {
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
) -> Result<
    (
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        http::Response<Option<Vec<u8>>>,
    ),
    Error,
>
where
    R: IntoClientRequest + Unpin,
{
    connect_async_with_config(request, None, false).await
}

#[cfg(feature = "connect")]
pub async fn connect_async_with_config<R>(
    request: R,
    config: Option<WebSocketConfig>,
    disable_nagle: bool,
) -> Result<
    (
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        http::Response<Option<Vec<u8>>>,
    ),
    Error,
>
where
    R: IntoClientRequest + Unpin,
{
    let _ = disable_nagle;
    let request = request.into_client_request()?;
    let stream = TcpStream::connect(request_addr(&request)?)
        .await
        .map_err(|error| Error::Io(std::io::Error::other(error.to_string())))?;
    let stream = MaybeTlsStream::plain(stream);
    let (ws, response) = edgerun_tungstenite::client_with_config(request, stream, config)?;
    Ok((WebSocketStream::direct(ws), response))
}

#[cfg(feature = "handshake")]
pub async fn client_async<R, S>(
    request: R,
    stream: S,
) -> Result<(WebSocketStream<S>, http::Response<Option<Vec<u8>>>), Error>
where
    R: IntoClientRequest + Unpin,
    S: Read + Write,
{
    client_async_with_config(request, stream, None).await
}

#[cfg(feature = "handshake")]
pub async fn client_async_with_config<R, S>(
    request: R,
    stream: S,
    config: Option<WebSocketConfig>,
) -> Result<(WebSocketStream<S>, http::Response<Option<Vec<u8>>>), Error>
where
    R: IntoClientRequest + Unpin,
    S: Read + Write,
{
    edgerun_tungstenite::client_with_config(request, stream, config)
        .map(|(ws, response)| (WebSocketStream::direct(ws), response))
}

#[cfg(feature = "handshake")]
pub async fn accept_async<S>(stream: S) -> Result<WebSocketStream<S>, Error>
where
    S: Read + Write,
{
    edgerun_tungstenite::accept(stream).map(WebSocketStream::direct)
}

#[cfg(feature = "handshake")]
pub async fn accept_async_with_config<S>(
    stream: S,
    config: Option<WebSocketConfig>,
) -> Result<WebSocketStream<S>, Error>
where
    S: Read + Write,
{
    edgerun_tungstenite::accept_with_config(stream, config).map(WebSocketStream::direct)
}

#[cfg(feature = "handshake")]
pub async fn accept_hdr_async<S, C>(stream: S, callback: C) -> Result<WebSocketStream<S>, Error>
where
    S: Read + Write,
    C: edgerun_tungstenite::handshake::server::Callback,
{
    accept_hdr_async_with_config(stream, callback, None).await
}

#[cfg(feature = "handshake")]
pub async fn accept_hdr_async_with_config<S, C>(
    stream: S,
    callback: C,
    config: Option<WebSocketConfig>,
) -> Result<WebSocketStream<S>, Error>
where
    S: Read + Write,
    C: edgerun_tungstenite::handshake::server::Callback,
{
    edgerun_tungstenite::accept_hdr_with_config(stream, callback, config)
        .map(WebSocketStream::direct)
}
