//! Compatibility surface for crates that currently use `tokio-tungstenite`.
//!
//! This crate owns the async boundary used by Edgerun Codex and converts to the
//! temporary upstream backend internally.

use backend::tungstenite as backend_ws;
use edgerun_tungstenite::protocol::CloseFrame;
use edgerun_tungstenite::protocol::WebSocketConfig;
use futures_util::Sink;
use futures_util::Stream;
use std::pin::Pin;
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

#[doc(hidden)]
pub mod backend {
    pub use tokio_tungstenite::Connector;
    pub use tokio_tungstenite::MaybeTlsStream;
    pub use tokio_tungstenite::WebSocketStream;
    pub use tokio_tungstenite::accept_async;
    pub use tokio_tungstenite::accept_async_with_config;
    pub use tokio_tungstenite::accept_hdr_async;
    pub use tokio_tungstenite::accept_hdr_async_with_config;
    pub use tokio_tungstenite::connect_async_tls_with_config;
    pub use tokio_tungstenite::tungstenite;
}

#[non_exhaustive]
#[derive(Clone)]
pub enum Connector {
    Plain,
    Rustls(Arc<rustls::ClientConfig>),
}

impl From<Connector> for backend::Connector {
    fn from(value: Connector) -> Self {
        match value {
            Connector::Plain => backend::Connector::Plain,
            Connector::Rustls(config) => backend::Connector::Rustls(config),
        }
    }
}

#[derive(Debug)]
pub struct MaybeTlsStream<S>(backend::MaybeTlsStream<S>);

impl<S> MaybeTlsStream<S> {
    pub fn get_ref(&self) -> &S {
        self.0.get_ref()
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for MaybeTlsStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

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
    TcpMaybeTls(backend::WebSocketStream<backend::MaybeTlsStream<tokio::net::TcpStream>>),
}

impl<S> WebSocketStream<S> {
    fn direct(inner: backend::WebSocketStream<S>) -> Self {
        Self(WebSocketStreamInner::Direct(inner))
    }
}

impl WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>> {
    fn tcp_maybe_tls(
        inner: backend::WebSocketStream<backend::MaybeTlsStream<tokio::net::TcpStream>>,
    ) -> Self {
        Self(WebSocketStreamInner::TcpMaybeTls(inner))
    }
}

impl<S> WebSocketStream<S> {
    pub async fn close(&mut self, frame: Option<CloseFrame>) -> Result<(), Error>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        match &mut self.0 {
            WebSocketStreamInner::Direct(inner) => inner
                .close(frame.map(Into::into))
                .await
                .map_err(Error::from),
            WebSocketStreamInner::TcpMaybeTls(inner) => inner
                .close(frame.map(Into::into))
                .await
                .map_err(Error::from),
        }
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
            WebSocketStreamInner::TcpMaybeTls(inner) => Pin::new(inner)
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
            WebSocketStreamInner::TcpMaybeTls(inner) => {
                Pin::new(inner).poll_ready(cx).map_err(Error::from)
            }
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        match &mut self.0 {
            WebSocketStreamInner::Direct(inner) => {
                Pin::new(inner).start_send(item.into()).map_err(Error::from)
            }
            WebSocketStreamInner::TcpMaybeTls(inner) => {
                Pin::new(inner).start_send(item.into()).map_err(Error::from)
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut self.0 {
            WebSocketStreamInner::Direct(inner) => {
                Pin::new(inner).poll_flush(cx).map_err(Error::from)
            }
            WebSocketStreamInner::TcpMaybeTls(inner) => {
                Pin::new(inner).poll_flush(cx).map_err(Error::from)
            }
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut self.0 {
            WebSocketStreamInner::Direct(inner) => {
                Pin::new(inner).poll_close(cx).map_err(Error::from)
            }
            WebSocketStreamInner::TcpMaybeTls(inner) => {
                Pin::new(inner).poll_close(cx).map_err(Error::from)
            }
        }
    }
}

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
    Ok((WebSocketStream::tcp_maybe_tls(stream), response))
}

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
    connect_async_tls_with_config(request, None, false, None).await
}

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
    connect_async_tls_with_config(request, config, disable_nagle, None).await
}

pub async fn accept_async<S>(stream: S) -> Result<WebSocketStream<S>, Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    backend::accept_async(stream)
        .await
        .map(WebSocketStream::direct)
        .map_err(Error::from)
}

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
