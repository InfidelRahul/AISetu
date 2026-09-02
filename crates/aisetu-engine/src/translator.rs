//! Bidirectional conversation translators.

use aisetu_conversation::{ConversationRequest, ConversationResponse, FinishReason, Message};
use aisetu_core::SetuError;
use serde_json::{json, Value};

use crate::{
    extractor::{Extractor, FallbackExtractor, JsonPathExtractor},
    normalizer::{Normalizer, WhitespaceNormalizer},
    validator::{validate_request_conversation, validate_response, SchemaValidator, Validator},
};

/// Opaque provider-side representation of a conversation turn.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderRepresentation {
    pub kind: String,
    pub payload: Value,
}

impl ProviderRepresentation {
    pub fn new(kind: impl Into<String>, payload: Value) -> Self {
        Self {
            kind: kind.into(),
            payload,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TranslationContext {
    pub provider: String,
    pub model: Option<String>,
}

pub trait RequestTranslator: Send + Sync {
    fn to_provider(
        &self,
        request: &ConversationRequest,
        ctx: &TranslationContext,
    ) -> aisetu_core::Result<ProviderRepresentation>;
}

pub trait ResponseTranslator: Send + Sync {
    fn decode(
        &self,
        representation: &ProviderRepresentation,
        ctx: &TranslationContext,
    ) -> aisetu_core::Result<ConversationResponse>;
}

/// Composite translator used by providers and tests.
pub struct ConversationTranslator {
    pub request: Box<dyn RequestTranslator>,
    pub response: Box<dyn ResponseTranslator>,
    pub normalizer: Box<dyn Normalizer>,
}

impl ConversationTranslator {
    pub fn mock() -> Self {
        Self {
            request: Box::new(MockRequestTranslator),
            response: Box::new(MockResponseTranslator::default()),
            normalizer: Box::new(WhitespaceNormalizer),
        }
    }

    pub fn translate_request(
        &self,
        request: &ConversationRequest,
        ctx: &TranslationContext,
    ) -> aisetu_core::Result<ProviderRepresentation> {
        let mut normalized = request.clone();
        normalized.conversation = self
            .normalizer
            .normalize_conversation(&request.conversation);
        validate_request_conversation(&normalized.conversation)?;
        self.request.to_provider(&normalized, ctx)
    }

    pub fn translate_response(
        &self,
        representation: &ProviderRepresentation,
        ctx: &TranslationContext,
    ) -> aisetu_core::Result<ConversationResponse> {
        let mut response = self.response.decode(representation, ctx)?;
        let text = self.normalizer.normalize_text(response.text());
        response.message = Message::assistant(text);
        validate_response(&response)?;
        Ok(response)
    }

    pub fn roundtrip(
        &self,
        request: &ConversationRequest,
        ctx: &TranslationContext,
        mut apply: impl FnMut(ProviderRepresentation) -> aisetu_core::Result<ProviderRepresentation>,
    ) -> aisetu_core::Result<ConversationResponse> {
        let outbound = self.translate_request(request, ctx)?;
        let inbound = apply(outbound)?;
        self.translate_response(&inbound, ctx)
    }
}

pub struct MockRequestTranslator;

impl RequestTranslator for MockRequestTranslator {
    fn to_provider(
        &self,
        request: &ConversationRequest,
        ctx: &TranslationContext,
    ) -> aisetu_core::Result<ProviderRepresentation> {
        let messages: Vec<Value> = request
            .conversation
            .messages
            .iter()
            .map(|m| {
                json!({
                    "role": m.role.as_str(),
                    "content": m.content.as_str(),
                })
            })
            .collect();
        Ok(ProviderRepresentation::new(
            "mock",
            json!({
                "provider": ctx.provider,
                "model": request.model.clone().or_else(|| ctx.model.clone()),
                "messages": messages,
            }),
        ))
    }
}

pub struct MockResponseTranslator {
    extractor: FallbackExtractor,
    validator: SchemaValidator,
}

impl Default for MockResponseTranslator {
    fn default() -> Self {
        Self {
            extractor: FallbackExtractor::new(vec![
                Box::new(JsonPathExtractor::new("message.content")),
                Box::new(JsonPathExtractor::new("text")),
                Box::new(JsonPathExtractor::new("content")),
                Box::new(JsonPathExtractor::new("choices.0.message.content")),
            ]),
            validator: SchemaValidator::new(&[]),
        }
    }
}

impl ResponseTranslator for MockResponseTranslator {
    fn decode(
        &self,
        representation: &ProviderRepresentation,
        ctx: &TranslationContext,
    ) -> aisetu_core::Result<ConversationResponse> {
        if representation.kind != "mock" && representation.kind != "echo" {
            return Err(SetuError::parse_failure(format!(
                "unexpected provider kind '{}'",
                representation.kind
            )));
        }
        if !self.validator.required_keys.is_empty() {
            self.validator.validate_payload(&representation.payload)?;
        }
        if representation.payload.get("error").is_some() {
            return Err(SetuError::provider_failure(format!(
                "provider '{}' returned an error payload",
                ctx.provider
            )));
        }
        let text = self.extractor.extract_text(&representation.payload)?;
        if text.trim().is_empty() {
            return Err(SetuError::parse_failure(
                "extracted assistant text is empty",
            ));
        }
        let mut response = ConversationResponse::assistant(text);
        response.provider = Some(ctx.provider.clone());
        response.model = ctx.model.clone();
        response.finish_reason = FinishReason::Stop;
        Ok(response)
    }
}

/// Identity mapping used when the provider already speaks the canonical shape.
pub struct CanonicalJsonTranslator;

impl RequestTranslator for CanonicalJsonTranslator {
    fn to_provider(
        &self,
        request: &ConversationRequest,
        _ctx: &TranslationContext,
    ) -> aisetu_core::Result<ProviderRepresentation> {
        let payload = serde_json::to_value(request).map_err(|e| {
            SetuError::parse_failure(format!("failed to serialize conversation request: {e}"))
        })?;
        Ok(ProviderRepresentation::new("canonical", payload))
    }
}

impl ResponseTranslator for CanonicalJsonTranslator {
    fn decode(
        &self,
        representation: &ProviderRepresentation,
        ctx: &TranslationContext,
    ) -> aisetu_core::Result<ConversationResponse> {
        let mut response: ConversationResponse =
            serde_json::from_value(representation.payload.clone()).map_err(|e| {
                SetuError::parse_failure(format!(
                    "failed to deserialize conversation response: {e}"
                ))
            })?;
        if response.provider.is_none() {
            response.provider = Some(ctx.provider.clone());
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aisetu_conversation::Conversation;

    fn sample_request() -> ConversationRequest {
        ConversationRequest::new(Conversation::with_messages(vec![
            Message::system("be brief"),
            Message::user("hello"),
        ]))
        .with_model("mock-text")
    }

    #[test]
    fn mock_roundtrip() {
        let engine = ConversationTranslator::mock();
        let ctx = TranslationContext {
            provider: "mock".into(),
            model: Some("mock-text".into()),
        };
        let response = engine
            .roundtrip(&sample_request(), &ctx, |rep| {
                assert_eq!(rep.kind, "mock");
                assert_eq!(rep.payload["messages"][1]["content"], "hello");
                Ok(ProviderRepresentation::new(
                    "mock",
                    json!({"text": "  hi there  "}),
                ))
            })
            .unwrap();
        assert_eq!(response.text(), "hi there");
        assert_eq!(response.provider.as_deref(), Some("mock"));
    }

    #[test]
    fn empty_extraction_fails() {
        let engine = ConversationTranslator::mock();
        let ctx = TranslationContext {
            provider: "mock".into(),
            model: None,
        };
        let err = engine
            .translate_response(
                &ProviderRepresentation::new("mock", json!({"text": "  "})),
                &ctx,
            )
            .unwrap_err();
        assert_eq!(err.kind, aisetu_core::ErrorKind::ParseFailure);
    }

    #[test]
    fn error_payload_fails() {
        let engine = ConversationTranslator::mock();
        let ctx = TranslationContext {
            provider: "mock".into(),
            model: None,
        };
        let err = engine
            .translate_response(
                &ProviderRepresentation::new("mock", json!({"error": "nope"})),
                &ctx,
            )
            .unwrap_err();
        assert_eq!(err.kind, aisetu_core::ErrorKind::ProviderFailure);
    }

    #[test]
    fn unexpected_fields_are_ignored() {
        let engine = ConversationTranslator::mock();
        let ctx = TranslationContext {
            provider: "mock".into(),
            model: Some("m".into()),
        };
        let rep = ProviderRepresentation::new(
            "mock",
            json!({"text": "ok", "unexpected": {"nested": true}, "foo": 1}),
        );
        let response = engine.translate_response(&rep, &ctx).unwrap();
        assert_eq!(response.text(), "ok");
    }
}
