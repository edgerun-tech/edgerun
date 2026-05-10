//! Compatibility surface for crates that currently use `tokio-tungstenite`.
//!
//! This crate owns the async boundary used by Edgerun Codex and converts to the
//! temporary upstream backend internally.

use backend::tungstenite as backend_ws;
use edgerun_tungstenite::protocol::CloseFrame;
use edgerun_tungstenite::protocol::Role;
use edgerun_tungstenite::protocol::WebSocketConfig;
use futures_util::Sink;
use futures_util::Stream;
use std::pin::Pin;
#[cfg(feature = "rustls-tls-native-roots")]
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::io::ReadBuf;

pub mod tungstenite {
    pub use edgerun_tungstenite::*;
}

pub use edgerun_tungstenite::Error;
pub use edgerun_tungstenite::Message;

mod backend {
    #[cfg(any(feature = "connect", feature = "rustls-tls-native-roots"))]
    pub use tokio_tungstenite::Connector;
    #[cfg(feature = "stream")]
    pub use tokio_tungstenite::MaybeTlsStream;
    pub use tokio_tungstenite::WebSocketStream;
    #[cfg(feature = "handshake")]
    pub use tokio_tungstenite::accept_async;
    #[cfg(feature = "handshake")]
    pub use tokio_tungstenite::accept_async_with_config;
    #[cfg(feature = "handshake")]
    pub use tokio_tungstenite::accept_hdr_async;
    #[cfg(feature = "handshake")]
    pub use tokio_tungstenite::accept_hdr_async_with_config;
    #[cfg(feature = "rustls-tls-native-roots")]
    pub use tokio_tungstenite::client_async_tls_with_config;
    #[cfg(feature = "handshake")]
    pub use tokio_tungstenite::client_async_with_config;
    #[cfg(all(feature = "connect", feature = "rustls-tls-native-roots"))]
    pub use tokio_tungstenite::connect_async_tls_with_config;
    #[cfg(feature = "connect")]
    pub use tokio_tungstenite::connect_async_with_config;
    pub use tokio_tungstenite::tungstenite;
}

#[non_exhaustive]
#[derive(Clone)]
pub enum Connector {
    Plain,
    #[cfg(feature = "rustls-tls-native-roots")]
    Rustls(Arc<rustls::ClientConfig>),
}

#[cfg(any(feature = "connect", feature = "rustls-tls-native-roots"))]
impl From<Connector> for backend::Connector {
    fn from(value: Connector) -> Self {
        match value {
            Connector::Plain => backend::Connector::Plain,
            #[cfg(feature = "rustls-tls-native-roots")]
            Connector::Rustls(config) => backend::Connector::Rustls(config),
        }
    }
}

#[cfg(feature = "stream")]
#[derive(Debug)]
pub struct MaybeTlsStream<S>(backend::MaybeTlsStream<S>);

#[cfg(feature = "stream")]
impl<S> MaybeTlsStream<S> {
    pub fn get_ref(&self) -> &S {
        self.0.get_ref()
    }

    pub fn get_mut(&mut self) -> &mut S {
        match &mut self.0 {
            backend::MaybeTlsStream::Plain(stream) => stream,
            #[cfg(feature = "rustls-tls-native-roots")]
            backend::MaybeTlsStream::Rustls(stream) => stream.get_mut().0,
            _ => unreachable!("unsupported backend TLS stream variant"),
        }
    }
}

#[cfg(feature = "stream")]
pub trait MaybeTlsStreamExt<S> {
    fn is_plain(&self) -> bool;
    fn plain_ref(&self) -> Option<&S>;
    fn plain_mut(&mut self) -> Option<&mut S>;
}

#[cfg(feature = "stream")]
impl<S> MaybeTlsStreamExt<S> for MaybeTlsStream<S> {
    fn is_plain(&self) -> bool {
        matches!(self.0, backend::MaybeTlsStream::Plain(_))
    }

    fn plain_ref(&self) -> Option<&S> {
        match &self.0 {
            backend::MaybeTlsStream::Plain(stream) => Some(stream),
            _ => None,
        }
    }

    fn plain_mut(&mut self) -> Option<&mut S> {
        match &mut self.0 {
            backend::MaybeTlsStream::Plain(stream) => Some(stream),
            _ => None,
        }
    }
}

#[cfg(feature = "stream")]
impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for MaybeTlsStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

#[cfg(feature = "stream")]
impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for MaybeTlsStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

#[derive(Debug)]
pub struct WebSocketStream<S>(WebSocketStreamInner<S>);

#[derive(Debug)]
enum WebSocketStreamInner<S> {
    Direct(backend::WebSocketStream<S>),
}

impl<S> WebSocketStream<S> {
    fn direct(inner: backend::WebSocketStream<S>) -> Self {
        Self(WebSocketStreamInner::Direct(inner))
    }
}

#[cfg(feature = "stream")]
impl<S> WebSocketStream<MaybeTlsStream<S>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    async fn from_backend_maybe_tls(
        inner: backend::WebSocketStream<backend::MaybeTlsStream<S>>,
    ) -> Self {
        let config = inner.get_config().into();
        let stream = MaybeTlsStream(inner.into_inner());
        Self::from_raw_socket(stream, Role::Client, Some(config)).await
    }
}

impl<S> WebSocketStream<S> {
    pub fn into_inner(self) -> S {
        match self.0 {
            WebSocketStreamInner::Direct(inner) => inner.into_inner(),
        }
    }

    pub fn get_ref(&self) -> &S
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        match &self.0 {
            WebSocketStreamInner::Direct(inner) => inner.get_ref(),
        }
    }

    pub fn get_mut(&mut self) -> &mut S
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        match &mut self.0 {
            WebSocketStreamInner::Direct(inner) => inner.get_mut(),
        }
    }

    pub fn get_config(&self) -> WebSocketConfig {
        match &self.0 {
            WebSocketStreamInner::Direct(inner) => inner.get_config().into(),
        }
    }

    pub async fn close(&mut self, frame: Option<CloseFrame>) -> Result<(), Error>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        match &mut self.0 {
            WebSocketStreamInner::Direct(inner) => inner
                .close(frame.map(Into::into))
                .await
                .map_err(Error::from),
        }
    }

    pub async fn from_raw_socket(stream: S, role: Role, config: Option<WebSocketConfig>) -> Self
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        Self::direct(
            backend::WebSocketStream::from_raw_socket(stream, role.into(), config.map(Into::into))
                .await,
        )
    }

    pub async fn from_partially_read(
        stream: S,
        part: Vec<u8>,
        role: Role,
        config: Option<WebSocketConfig>,
    ) -> Self
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        Self::direct(
            backend::WebSocketStream::from_partially_read(
                stream,
                part,
                role.into(),
                config.map(Into::into),
            )
            .await,
        )
    }
}

impl<S> Stream for WebSocketStream<S>
where
    backend::WebSocketStream<S>:
        Stream<Item = Result<backend_ws::Message, backend_ws::Error>> + Unpin,
{
    type Item = Result<Message, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &mut self.0 {
            WebSocketStreamInner::Direct(inner) => Pin::new(inner)
                .poll_next(cx)
                .map(|option| option.map(|result| result.map(Message::from).map_err(Error::from))),
        }
    }
}

impl<S> Sink<Message> for WebSocketStream<S>
where
    backend::WebSocketStream<S>: Sink<backend_ws::Message, Error = backend_ws::Error> + Unpin,
{
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut self.0 {
            WebSocketStreamInner::Direct(inner) => {
                Pin::new(inner).poll_ready(cx).map_err(Error::from)
            }
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        match &mut self.0 {
            WebSocketStreamInner::Direct(inner) => {
                Pin::new(inner).start_send(item.into()).map_err(Error::from)
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut self.0 {
            WebSocketStreamInner::Direct(inner) => {
                Pin::new(inner).poll_flush(cx).map_err(Error::from)
            }
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut self.0 {
            WebSocketStreamInner::Direct(inner) => {
                Pin::new(inner).poll_close(cx).map_err(Error::from)
            }
        }
    }
}

#[cfg(all(feature = "connect", feature = "rustls-tls-native-roots"))]
pub async fn connect_async_tls_with_config<R>(
    request: R,
    config: Option<WebSocketConfig>,
    disable_nagle: bool,
    connector: Option<Connector>,
) -> Result<
    (
        WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
        http::Response<Option<Vec<u8>>>,
    ),
    Error,
>
where
    R: edgerun_tungstenite::client::IntoClientRequest + Unpin,
{
    let request = request.into_client_request()?;
    let (stream, response) = backend::connect_async_tls_with_config(
        request,
        config.map(Into::into),
        disable_nagle,
        connector.map(Into::into),
    )
    .await
    .map_err(Error::from)?;
    Ok((
        WebSocketStream::from_backend_maybe_tls(stream).await,
        response,
    ))
}

#[cfg(feature = "connect")]
pub async fn connect_async<R>(
    request: R,
) -> Result<
    (
        WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
        http::Response<Option<Vec<u8>>>,
    ),
    Error,
>
where
    R: edgerun_tungstenite::client::IntoClientRequest + Unpin,
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
        WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
        http::Response<Option<Vec<u8>>>,
    ),
    Error,
>
where
    R: edgerun_tungstenite::client::IntoClientRequest + Unpin,
{
    let request = request.into_client_request()?;
    let (stream, response) =
        backend::connect_async_with_config(request, config.map(Into::into), disable_nagle)
            .await
            .map_err(Error::from)?;
    Ok((
        WebSocketStream::from_backend_maybe_tls(stream).await,
        response,
    ))
}

#[cfg(feature = "rustls-tls-native-roots")]
pub async fn client_async_tls<R, S>(
    request: R,
    stream: S,
) -> Result<
    (
        WebSocketStream<MaybeTlsStream<S>>,
        http::Response<Option<Vec<u8>>>,
    ),
    Error,
>
where
    R: edgerun_tungstenite::client::IntoClientRequest + Unpin,
    S: 'static + tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin,
    MaybeTlsStream<S>: Unpin,
{
    client_async_tls_with_config(request, stream, None, None).await
}

#[cfg(feature = "rustls-tls-native-roots")]
pub async fn client_async_tls_with_config<R, S>(
    request: R,
    stream: S,
    config: Option<WebSocketConfig>,
    connector: Option<Connector>,
) -> Result<
    (
        WebSocketStream<MaybeTlsStream<S>>,
        http::Response<Option<Vec<u8>>>,
    ),
    Error,
>
where
    R: edgerun_tungstenite::client::IntoClientRequest + Unpin,
    S: 'static + tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin,
    MaybeTlsStream<S>: Unpin,
{
    let request = request.into_client_request()?;
    let (stream, response) = backend::client_async_tls_with_config(
        request,
        stream,
        config.map(Into::into),
        connector.map(Into::into),
    )
    .await
    .map_err(Error::from)?;
    Ok((
        WebSocketStream::from_backend_maybe_tls(stream).await,
        response,
    ))
}

#[cfg(feature = "handshake")]
pub async fn client_async<R, S>(
    request: R,
    stream: S,
) -> Result<(WebSocketStream<S>, http::Response<Option<Vec<u8>>>), Error>
where
    R: edgerun_tungstenite::client::IntoClientRequest + Unpin,
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
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
    R: edgerun_tungstenite::client::IntoClientRequest + Unpin,
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let request = request.into_client_request()?;
    let (stream, response) =
        backend::client_async_with_config(request, stream, config.map(Into::into))
            .await
            .map_err(Error::from)?;
    Ok((WebSocketStream::direct(stream), response))
}

#[cfg(feature = "handshake")]
pub async fn accept_async<S>(stream: S) -> Result<WebSocketStream<S>, Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    backend::accept_async(stream)
        .await
        .map(WebSocketStream::direct)
        .map_err(Error::from)
}

#[cfg(feature = "handshake")]
pub async fn accept_async_with_config<S>(
    stream: S,
    config: Option<WebSocketConfig>,
) -> Result<WebSocketStream<S>, Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    backend::accept_async_with_config(stream, config.map(Into::into))
        .await
        .map(WebSocketStream::direct)
        .map_err(Error::from)
}

#[cfg(feature = "handshake")]
pub async fn accept_hdr_async<S, C>(stream: S, callback: C) -> Result<WebSocketStream<S>, Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    C: FnOnce(
            &http::Request<()>,
            http::Response<()>,
        ) -> Result<http::Response<()>, http::Response<Option<String>>>
        + Unpin,
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
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    C: FnOnce(
            &http::Request<()>,
            http::Response<()>,
        ) -> Result<http::Response<()>, http::Response<Option<String>>>
        + Unpin,
{
    backend::accept_hdr_async_with_config(stream, callback, config.map(Into::into))
        .await
        .map(WebSocketStream::direct)
        .map_err(Error::from)
}
