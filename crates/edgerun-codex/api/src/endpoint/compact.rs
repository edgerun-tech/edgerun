use crate::auth::SharedAuthProvider;
use crate::common::CompactionInput;
use crate::endpoint::session::EndpointSession;
use crate::error::ApiError;
use crate::provider::Provider;
use codex_client::HttpTransport;
use codex_client::RequestTelemetry;
use codex_protocol::models::ResponseItem;
use edgerun_http::HeaderMap;
use edgerun_http::Method;
use edgerun_json::ToJson;
use std::sync::Arc;

pub struct CompactClient<T: HttpTransport> {
    session: EndpointSession<T>,
}

impl<T: HttpTransport> CompactClient<T> {
    pub fn new(transport: T, provider: Provider, auth: SharedAuthProvider) -> Self {
        Self {
            session: EndpointSession::new(transport, provider, auth),
        }
    }

    pub fn with_telemetry(self, request: Option<Arc<dyn RequestTelemetry>>) -> Self {
        Self {
            session: self.session.with_request_telemetry(request),
        }
    }

    fn path() -> &'static str {
        "responses/compact"
    }

    pub async fn compact(
        &self,
        body: edgerun_json::Value,
        extra_headers: HeaderMap,
    ) -> Result<Vec<ResponseItem>, ApiError> {
        let resp = self
            .session
            .execute(Method::POST, Self::path(), extra_headers, Some(body))
            .await?;
        let parsed =
            edgerun_json::from_slice(&resp.body).map_err(|e| ApiError::Stream(e.to_string()))?;
        let output = parsed
            .get("output")
            .cloned()
            .ok_or_else(|| ApiError::Stream("compact response missing output".to_string()))?;
        edgerun_json::from_json_value(output).map_err(|e| ApiError::Stream(e.to_string()))
    }

    pub async fn compact_input(
        &self,
        input: &CompactionInput<'_>,
        extra_headers: HeaderMap,
    ) -> Result<Vec<ResponseItem>, ApiError> {
        let body = input.to_json();
        self.compact(body, extra_headers).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_client::Request;
    use codex_client::Response;
    use codex_client::StreamResponse;
    use codex_client::TransportError;

    #[derive(Clone, Default)]
    struct DummyTransport;

    impl HttpTransport for DummyTransport {
        fn execute<'async_trait>(
            &'async_trait self,
            _req: Request,
        ) -> core::pin::Pin<
            Box<
                dyn core::future::Future<Output = Result<Response, TransportError>>
                    + Send
                    + 'async_trait,
            >,
        >
        where
            Self: 'async_trait,
        {
            Box::pin(
                async move { Err(TransportError::Build("execute should not run".to_string())) },
            )
        }

        fn stream<'async_trait>(
            &'async_trait self,
            _req: Request,
        ) -> core::pin::Pin<
            Box<
                dyn core::future::Future<Output = Result<StreamResponse, TransportError>>
                    + Send
                    + 'async_trait,
            >,
        >
        where
            Self: 'async_trait,
        {
            Box::pin(async move { Err(TransportError::Build("stream should not run".to_string())) })
        }
    }

    #[test]
    fn path_is_responses_compact() {
        assert_eq!(CompactClient::<DummyTransport>::path(), "responses/compact");
    }
}
