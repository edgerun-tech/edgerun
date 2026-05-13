#[cfg(feature = "reqwest-transport")]
use crate::default_client::CodexHttpClient;
#[cfg(feature = "reqwest-transport")]
use crate::default_client::CodexRequestBuilder;
use crate::error::TransportError;
use crate::request::Request;
#[cfg(feature = "reqwest-transport")]
use crate::request::RequestBody;
use crate::request::Response;
use edgerun_async_trait::async_trait;
use edgerun_bytes::Bytes;
#[cfg(feature = "reqwest-transport")]
use edgerun_futures::StreamExt;
use edgerun_futures::stream::BoxStream;
use edgerun_http::HeaderMap;
#[cfg(feature = "reqwest-transport")]
use edgerun_http::Method;
use edgerun_http::StatusCode;
use std::sync::Arc;
#[cfg(feature = "reqwest-transport")]
use tracing::Level;
#[cfg(feature = "reqwest-transport")]
use tracing::enabled;
#[cfg(feature = "reqwest-transport")]
use tracing::trace;

pub type ByteStream = BoxStream<'static, Result<Bytes, TransportError>>;

pub struct StreamResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub bytes: ByteStream,
}

#[async_trait]
pub trait HttpTransport: Send + Sync {
    async fn execute(&self, req: Request) -> Result<Response, TransportError>;
    async fn stream(&self, req: Request) -> Result<StreamResponse, TransportError>;
}

#[async_trait]
impl<T> HttpTransport for Arc<T>
where
    T: HttpTransport + ?Sized,
{
    async fn execute(&self, req: Request) -> Result<Response, TransportError> {
        self.as_ref().execute(req).await
    }

    async fn stream(&self, req: Request) -> Result<StreamResponse, TransportError> {
        self.as_ref().stream(req).await
    }
}

#[cfg(feature = "reqwest-transport")]
#[derive(Clone, Debug)]
pub struct ReqwestTransport {
    client: CodexHttpClient,
}

#[cfg(feature = "reqwest-transport")]
impl ReqwestTransport {
    pub fn new(client: edgerun_reqwest::Client) -> Self {
        Self {
            client: CodexHttpClient::new(client),
        }
    }

    fn build(&self, req: Request) -> Result<CodexRequestBuilder, TransportError> {
        let prepared = req.prepare_body_for_send().map_err(TransportError::Build)?;

        let Request {
            method,
            url,
            headers: _,
            body: _,
            compression: _,
            timeout,
        } = req;

        let mut builder = self.client.request(
            Method::from_bytes(method.as_str().as_bytes()).unwrap_or(Method::GET),
            &url,
        );

        if let Some(timeout) = timeout {
            builder = builder.timeout(timeout);
        }

        builder = builder.headers(prepared.headers);
        if let Some(body) = prepared.body {
            builder = builder.body(body);
        }
        Ok(builder)
    }

    fn map_error(err: edgerun_reqwest::Error) -> TransportError {
        if err.is_timeout() {
            TransportError::Timeout
        } else {
            TransportError::Network(err.to_string())
        }
    }
}

#[cfg(feature = "reqwest-transport")]
fn request_body_for_trace(req: &Request) -> String {
    match req.body.as_ref() {
        Some(RequestBody::Json(body)) => body.to_string(),
        Some(RequestBody::Raw(body)) => format!("<raw body: {} bytes>", body.len()),
        None => String::new(),
    }
}

#[cfg(feature = "reqwest-transport")]
#[async_trait]
impl HttpTransport for ReqwestTransport {
    async fn execute(&self, req: Request) -> Result<Response, TransportError> {
        if enabled!(Level::TRACE) {
            trace!(
                "{} to {}: {}",
                req.method,
                req.url,
                request_body_for_trace(&req)
            );
        }

        let url = req.url.clone();
        let builder = self.build(req)?;
        let resp = builder.send().await.map_err(Self::map_error)?;
        let status = resp.status();
        let headers = resp.headers().clone();
        let bytes = resp.bytes().await.map_err(Self::map_error)?;
        if !status.is_success() {
            let body = String::from_utf8(bytes.to_vec()).ok();
            return Err(TransportError::Http {
                status,
                url: Some(url),
                headers: Some(headers),
                body,
            });
        }
        Ok(Response {
            status,
            headers,
            body: bytes,
        })
    }

    async fn stream(&self, req: Request) -> Result<StreamResponse, TransportError> {
        if enabled!(Level::TRACE) {
            trace!(
                "{} to {}: {}",
                req.method,
                req.url,
                request_body_for_trace(&req)
            );
        }

        let url = req.url.clone();
        let builder = self.build(req)?;
        let resp = builder.send().await.map_err(Self::map_error)?;
        let status = resp.status();
        let headers = resp.headers().clone();
        if !status.is_success() {
            let body = resp.text().await.ok();
            return Err(TransportError::Http {
                status,
                url: Some(url),
                headers: Some(headers),
                body,
            });
        }
        let stream = resp
            .bytes_stream()
            .map(|result| result.map_err(Self::map_error));
        Ok(StreamResponse {
            status,
            headers,
            bytes: Box::pin(stream),
        })
    }
}
