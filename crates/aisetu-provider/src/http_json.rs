//! First HTTP provider adapter.
//!
//! Posts a JSON conversation to `base_url` and extracts assistant text.
//! Session cookies/headers from Session Manager are applied to the transport.

use std::sync::Arc;
use std::time::Instant;

use aisetu_conversation::{ConversationRequest, ConversationResponse, Usage};
use aisetu_engine::{ConversationTranslator, ProviderRepresentation, TranslationContext};
use aisetu_session::Session;
use aisetu_transport::{HttpRequest, Transport};
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::info_span;
use tracing::Instrument;

use crate::{
    adapter::{Provider, ProviderId},
    capabilities::ProviderCapabilities,
    error::{normalize_http_error, normalize_transport_error},
    reliability::ReliableExtractor,
};

pub struct HttpJsonProvider {
    id: ProviderId,
    base_url: String,
    upstream_model: Option<String>,
    transport: Arc<dyn Transport>,
    translator: ConversationTranslator,
    extractor: ReliableExtractor,
    capabilities: ProviderCapabilities,
}

impl HttpJsonProvider {
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        transport: Arc<dyn Transport>,
    ) -> Self {
        Self {
            id: ProviderId::new(name),
            base_url: base_url.into(),
            upstream_model: None,
            transport,
            translator: ConversationTranslator::mock(),
            extractor: ReliableExtractor::default(),
            capabilities: ProviderCapabilities::text_only().with_streaming(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.upstream_model = Some(model.into());
        self
    }

    fn apply_session(&self, mut req: HttpRequest, session: Option<&Session>) -> HttpRequest {
        if let Some(session) = session {
            for (k, v) in session.cookies.iter() {
                req = req.cookie(k, v);
            }
            for (k, v) in session.headers.iter() {
                req = req.header(k, v);
            }
        }
        req
    }
}

#[async_trait]
impl Provider for HttpJsonProvider {
    fn id(&self) -> &ProviderId {
        &self.id
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn complete(
        &self,
        request: ConversationRequest,
        session: Option<&Session>,
    ) -> aisetu_core::Result<ConversationResponse> {
        request.validate()?;
        if session.is_some_and(|s| s.is_expired()) {
            return Err(aisetu_core::SetuError::session_expired(format!(
                "session for provider '{}' is expired",
                self.id
            )));
        }

        let ctx = TranslationContext {
            provider: self.id.as_str().to_string(),
            model: request
                .model
                .clone()
                .or_else(|| self.upstream_model.clone()),
        };

        let span = info_span!(
            "provider.complete",
            provider = %self.id,
            model = ctx.model.as_deref().unwrap_or(""),
        );

        async move {
            let started = Instant::now();
            let representation = self.translator.translate_request(&request, &ctx)?;
            let body = serde_json::to_string(&representation.payload).map_err(|e| {
                aisetu_core::SetuError::parse_failure(format!("serialize provider request: {e}"))
            })?;

            let http_req = self.apply_session(
                HttpRequest::post(&self.base_url)
                    .header("accept", "application/json")
                    .json(body),
                session,
            );

            let http_resp = self
                .transport
                .execute(http_req)
                .await
                .map_err(|e| normalize_transport_error(self.id.as_str(), e))?;

            tracing::debug!(
                status = http_resp.status.as_u16(),
                transport_ms = http_resp.elapsed_ms,
                "provider http complete"
            );

            if !http_resp.status.is_success() {
                return Err(normalize_http_error(self.id.as_str(), &http_resp));
            }

            let payload: Value = http_resp.json().or_else(|_| {
                Ok::<Value, aisetu_core::SetuError>(json!({"text": http_resp.text().unwrap_or("")}))
            })?;

            let inbound = ProviderRepresentation::new("http_json", payload);
            let mut response = match self.translator.translate_response(&inbound, &ctx) {
                Ok(r) => r,
                Err(err) if err.kind == aisetu_core::ErrorKind::ParseFailure => {
                    self.extractor.extract(&inbound).await?
                }
                Err(err) => return Err(err),
            };
            response.provider = Some(self.id.as_str().to_string());
            response.model = ctx.model.clone();
            if response.usage.is_none() {
                response.usage = Some(Usage::new(0, 0));
            }
            tracing::debug!(
                provider_ms = started.elapsed().as_millis() as u64,
                "provider conversation complete"
            );
            Ok(response)
        }
        .instrument(span)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aisetu_conversation::{Conversation, Message};
    use aisetu_transport::client::MockTransport;

    #[tokio::test]
    async fn http_json_roundtrip() {
        let transport = Arc::new(MockTransport::json_ok(r#"{"text":"from provider"}"#));
        let provider = HttpJsonProvider::new("web", "https://provider.example/chat", transport);
        let req =
            ConversationRequest::new(Conversation::with_messages(vec![Message::user("hello")]));
        let resp = provider.complete(req, None).await.unwrap();
        assert_eq!(resp.text(), "from provider");
        assert_eq!(resp.provider.as_deref(), Some("web"));
    }

    #[tokio::test]
    async fn http_error_normalized() {
        let transport = Arc::new(MockTransport::new(|req| {
            Ok(aisetu_transport::HttpResponse {
                status: aisetu_transport::StatusCode(429),
                headers: Default::default(),
                body: aisetu_transport::Body::from_text(r#"{"error":{"message":"rate"}}"#),
                cookies: req.cookies,
                elapsed_ms: 1,
                url: req.url,
            })
        }));
        let provider = HttpJsonProvider::new("web", "https://provider.example/chat", transport);
        let req = ConversationRequest::new(Conversation::with_messages(vec![Message::user("x")]));
        let err = provider.complete(req, None).await.unwrap_err();
        assert_eq!(err.kind, aisetu_core::ErrorKind::RateLimited);
        assert_eq!(err.message, "rate");
    }
}
