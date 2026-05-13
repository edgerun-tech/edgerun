use edgerun_http::Error as HttpError;
use edgerun_http::HeaderMap;
use edgerun_http::HeaderName;
use edgerun_http::HeaderValue;
use edgerun_json::ToJson;
use edgerun_reqwest::IntoUrl;
use edgerun_reqwest::Method;
use edgerun_reqwest::Response;
use std::fmt::Display;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct CodexHttpClient {
    inner: edgerun_reqwest::Client,
}

impl CodexHttpClient {
    pub fn new(inner: edgerun_reqwest::Client) -> Self {
        Self { inner }
    }

    pub fn get<U>(&self, url: U) -> CodexRequestBuilder
    where
        U: IntoUrl,
    {
        self.request(Method::GET, url)
    }

    pub fn post<U>(&self, url: U) -> CodexRequestBuilder
    where
        U: IntoUrl,
    {
        self.request(Method::POST, url)
    }

    pub fn request<U>(&self, method: Method, url: U) -> CodexRequestBuilder
    where
        U: IntoUrl,
    {
        let url_str = url.as_str().to_string();
        CodexRequestBuilder::new(self.inner.request(method.clone(), url), method, url_str)
    }
}

#[must_use = "requests are not sent unless `send` is awaited"]
#[derive(Debug)]
pub struct CodexRequestBuilder {
    builder: edgerun_reqwest::RequestBuilder,
    method: Method,
    url: String,
}

impl CodexRequestBuilder {
    fn new(builder: edgerun_reqwest::RequestBuilder, method: Method, url: String) -> Self {
        Self {
            builder,
            method,
            url,
        }
    }

    fn map(
        self,
        f: impl FnOnce(edgerun_reqwest::RequestBuilder) -> edgerun_reqwest::RequestBuilder,
    ) -> Self {
        Self {
            builder: f(self.builder),
            method: self.method,
            url: self.url,
        }
    }

    pub fn headers(self, headers: HeaderMap) -> Self {
        self.map(|builder| builder.headers(headers))
    }

    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<HttpError>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<HttpError>,
    {
        self.map(|builder| builder.header(key, value))
    }

    pub fn bearer_auth<T>(self, token: T) -> Self
    where
        T: Display,
    {
        self.map(|builder| builder.bearer_auth(token))
    }

    pub fn timeout(self, timeout: Duration) -> Self {
        self.map(|builder| builder.timeout(timeout))
    }

    pub fn json<T>(self, value: &T) -> Self
    where
        T: ?Sized + ToJson,
    {
        self.map(|builder| builder.json_edgerun(value))
    }

    pub fn body<B>(self, body: B) -> Self
    where
        B: Into<edgerun_reqwest::Body>,
    {
        self.map(|builder| builder.body(body))
    }

    pub async fn send(self) -> Result<Response, edgerun_reqwest::Error> {
        let headers = trace_headers();

        match self.builder.headers(headers).send().await {
            Ok(response) => {
                tracing::debug!(
                    method = %self.method,
                    url = %self.url,
                    status = %response.status(),
                    headers = ?response.headers(),
                    version = ?response.version(),
                    "Request completed"
                );

                Ok(response)
            }
            Err(error) => {
                let status = error.status();
                tracing::debug!(
                    method = %self.method,
                    url = %self.url,
                    status = status.map(|s| s.as_u16()),
                    error = %error,
                    "Request failed"
                );
                Err(error)
            }
        }
    }
}

fn trace_headers() -> HeaderMap {
    HeaderMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_headers_are_empty_without_external_otel_bridge() {
        let headers = trace_headers();
        assert_eq!(headers.keys().count(), 0);
    }
}
