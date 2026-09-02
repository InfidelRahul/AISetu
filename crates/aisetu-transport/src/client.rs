//! HTTP transport client.

use std::time::{Duration, Instant};

use aisetu_core::{config::TransportConfig, SetuError};
use async_trait::async_trait;
use tracing::{debug, info_span, Instrument};

use crate::{
    request::{Body, HttpRequest, Method},
    response::{HttpResponse, StatusCode},
    HeaderMap,
};

/// Generic transport interface. Implementations may be real HTTP or in-memory mocks.
#[async_trait]
pub trait Transport: Send + Sync {
    async fn execute(&self, request: HttpRequest) -> aisetu_core::Result<HttpResponse>;
}

/// reqwest-backed HTTP transport with TLS, cookies, timeouts.
pub struct HttpTransport {
    client: reqwest::Client,
    default_timeout: Duration,
    max_response_bytes: usize,
}

impl HttpTransport {
    pub fn new(config: &TransportConfig) -> aisetu_core::Result<Self> {
        Self::with_limits(config, 8 * 1024 * 1024)
    }

    pub fn with_limits(
        config: &TransportConfig,
        max_response_bytes: usize,
    ) -> aisetu_core::Result<Self> {
        let mut builder = reqwest::Client::builder()
            .user_agent(&config.user_agent)
            .timeout(Duration::from_millis(config.timeout_ms))
            .connect_timeout(Duration::from_millis(config.connect_timeout_ms))
            .redirect(reqwest::redirect::Policy::limited(config.max_redirects))
            .gzip(true);

        if !config.tls_verify {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let client = builder
            .build()
            .map_err(|e| SetuError::configuration(format!("failed to build HTTP client: {e}")))?;

        Ok(Self {
            client,
            default_timeout: Duration::from_millis(config.timeout_ms),
            max_response_bytes,
        })
    }

    pub fn from_client(client: reqwest::Client) -> Self {
        Self {
            client,
            default_timeout: Duration::from_secs(60),
            max_response_bytes: 8 * 1024 * 1024,
        }
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn execute(&self, request: HttpRequest) -> aisetu_core::Result<HttpResponse> {
        request.validate()?;
        let span = info_span!(
            "http.execute",
            method = %request.method,
            url.host = tracing::field::Empty,
            status = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty,
        );
        self.execute_inner(request).instrument(span).await
    }
}

impl HttpTransport {
    async fn execute_inner(&self, request: HttpRequest) -> aisetu_core::Result<HttpResponse> {
        let current = tracing::Span::current();
        if let Ok(parsed) = url::Url::parse(&request.url) {
            if let Some(host) = parsed.host_str() {
                current.record("url.host", host);
            }
        }

        let method = map_method(request.method);
        let mut builder = self.client.request(method, &request.url);

        for (k, v) in request.headers.iter() {
            builder = builder.header(k, v);
        }
        if let Some(cookie) = request.cookies.header_value() {
            builder = builder.header("cookie", cookie);
        }

        if !request.body.is_empty() {
            builder = builder.body(request.body.as_bytes().to_vec());
        }

        let timeout = if request.timeout.total.as_millis() == 0 {
            self.default_timeout
        } else {
            request.timeout.total
        };
        builder = builder.timeout(timeout);

        let started = Instant::now();
        let response = builder.send().await.map_err(map_reqwest_error)?;
        let status = response.status().as_u16();
        let final_url = response.url().to_string();

        let mut headers = HeaderMap::new();
        let mut cookies = request.cookies.clone();
        for (name, value) in response.headers().iter() {
            let v = value.to_str().unwrap_or("").to_string();
            if name.as_str().eq_ignore_ascii_case("set-cookie") {
                cookies.absorb_set_cookie(&v);
            }
            headers.append(name.as_str().to_string(), v);
        }

        let bytes = response.bytes().await.map_err(map_reqwest_error)?;
        if bytes.len() > self.max_response_bytes {
            return Err(SetuError::resource_exhausted(format!(
                "response body {} bytes exceeds limit {}",
                bytes.len(),
                self.max_response_bytes
            )));
        }

        let elapsed_ms = started.elapsed().as_millis() as u64;
        current.record("status", status);
        current.record("elapsed_ms", elapsed_ms);
        debug!(bytes = bytes.len(), "http response received");

        let body = match String::from_utf8(bytes.to_vec()) {
            Ok(text) => Body::Text(text),
            Err(e) => Body::Bytes(e.into_bytes()),
        };

        Ok(HttpResponse {
            status: StatusCode(status),
            headers,
            body,
            cookies,
            elapsed_ms,
            url: final_url,
        })
    }
}

fn map_method(method: Method) -> reqwest::Method {
    match method {
        Method::Get => reqwest::Method::GET,
        Method::Post => reqwest::Method::POST,
        Method::Put => reqwest::Method::PUT,
        Method::Patch => reqwest::Method::PATCH,
        Method::Delete => reqwest::Method::DELETE,
        Method::Head => reqwest::Method::HEAD,
        Method::Options => reqwest::Method::OPTIONS,
    }
}

fn map_reqwest_error(err: reqwest::Error) -> SetuError {
    if err.is_timeout() {
        SetuError::timeout(format!("HTTP request timed out: {err}"))
    } else if err.is_connect() {
        SetuError::network(format!("HTTP connect failed: {err}"))
    } else if err.is_builder() {
        SetuError::invalid_request(format!("invalid HTTP request: {err}"))
    } else if err.is_redirect() {
        SetuError::network(format!("HTTP redirect error: {err}"))
    } else if err.is_decode() {
        SetuError::parse_failure(format!("HTTP decode error: {err}"))
    } else {
        SetuError::network(format!("HTTP error: {err}"))
    }
}

/// In-memory transport used by unit tests.
pub struct MockTransport {
    handler: Box<dyn Fn(HttpRequest) -> aisetu_core::Result<HttpResponse> + Send + Sync>,
}

impl MockTransport {
    pub fn new(
        handler: impl Fn(HttpRequest) -> aisetu_core::Result<HttpResponse> + Send + Sync + 'static,
    ) -> Self {
        Self {
            handler: Box::new(handler),
        }
    }

    pub fn json_ok(body: impl Into<String>) -> Self {
        let body = body.into();
        Self::new(move |req| {
            Ok(HttpResponse {
                status: StatusCode(200),
                headers: {
                    let mut h = HeaderMap::new();
                    h.set_content_type("application/json");
                    h
                },
                body: Body::from_text(body.clone()),
                cookies: req.cookies,
                elapsed_ms: 0,
                url: req.url,
            })
        })
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn execute(&self, request: HttpRequest) -> aisetu_core::Result<HttpResponse> {
        request.validate()?;
        (self.handler)(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HttpRequest;
    use aisetu_core::config::TransportConfig;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn mock_transport_roundtrip() {
        let t = MockTransport::json_ok(r#"{"hello":"world"}"#);
        let resp = t
            .execute(HttpRequest::get("https://example.com/ok"))
            .await
            .unwrap();
        assert!(resp.status.is_success());
        assert_eq!(resp.text().unwrap(), r#"{"hello":"world"}"#);
    }

    #[tokio::test]
    async fn real_http_against_wiremock() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ping"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("pong")
                    .insert_header("set-cookie", "sid=abc; Path=/"),
            )
            .mount(&server)
            .await;

        let transport = HttpTransport::new(&TransportConfig::default()).unwrap();
        let resp = transport
            .execute(HttpRequest::get(format!("{}/ping", server.uri())))
            .await
            .unwrap();
        assert_eq!(resp.status.as_u16(), 200);
        assert_eq!(resp.text().unwrap(), "pong");
        assert_eq!(resp.cookies.get("sid"), Some("abc"));
        assert!(resp.elapsed_ms < 10_000);
    }

    #[tokio::test]
    async fn timeout_is_mapped() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/slow"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
            .mount(&server)
            .await;

        let cfg = TransportConfig {
            timeout_ms: 50,
            connect_timeout_ms: 50,
            ..TransportConfig::default()
        };
        let transport = HttpTransport::new(&cfg).unwrap();
        let err = transport
            .execute(
                HttpRequest::get(format!("{}/slow", server.uri()))
                    .timeout(Duration::from_millis(50)),
            )
            .await
            .unwrap_err();
        assert_eq!(err.kind, aisetu_core::ErrorKind::Timeout);
    }
}
