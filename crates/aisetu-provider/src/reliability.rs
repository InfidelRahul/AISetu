//! Harden conversation extraction against malformed provider payloads.
//!
//! Needle is used when deterministic extraction is insufficient.

use aisetu_conversation::{ConversationResponse, Message};
use aisetu_engine::{
    extractor::{Extractor, FallbackExtractor, JsonPathExtractor, RegexExtractor},
    validator::validate_response,
    ProviderRepresentation,
};
use aisetu_intelligence::{
    IntelligenceContext, IntelligenceEngine, IntelligenceInput, JsonSchema, NeedleEngine,
};
use serde_json::Value;

pub struct ReliableExtractor {
    fallback: FallbackExtractor,
    needle: NeedleEngine,
}

impl Default for ReliableExtractor {
    fn default() -> Self {
        Self {
            fallback: FallbackExtractor::new(vec![
                Box::new(JsonPathExtractor::new("choices.0.message.content")),
                Box::new(JsonPathExtractor::new("message.content")),
                Box::new(JsonPathExtractor::new("text")),
                Box::new(JsonPathExtractor::new("content")),
                Box::new(JsonPathExtractor::new("output")),
                Box::new(JsonPathExtractor::new("response")),
                Box::new(RegexExtractor::between("\"content\":\"", "\"")),
            ]),
            needle: NeedleEngine::new(),
        }
    }
}

impl ReliableExtractor {
    pub async fn extract(
        &self,
        representation: &ProviderRepresentation,
    ) -> aisetu_core::Result<ConversationResponse> {
        if representation.payload.is_null() {
            return Err(aisetu_core::SetuError::parse_failure(
                "provider payload is null",
            ));
        }
        if let Some(err) = representation.payload.get("error") {
            if !err.is_null() {
                return Err(aisetu_core::SetuError::provider_failure(format!(
                    "provider error: {err}"
                )));
            }
        }

        match self.fallback.extract_text(&representation.payload) {
            Ok(text) if !text.trim().is_empty() => {
                let response = ConversationResponse::assistant(sanitize(&text));
                validate_response(&response)?;
                return Ok(response);
            }
            Ok(_) | Err(_) => {}
        }

        let schema = JsonSchema::object(&["text"], vec![("text", JsonSchema::string())]);
        let raw = match &representation.payload {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        let inferred = self
            .needle
            .infer(
                &IntelligenceInput::new(raw),
                &schema,
                &IntelligenceContext {
                    source: Some("provider-response".into()),
                    hints: vec!["text".into()],
                },
            )
            .await;
        match inferred {
            Ok(out) => {
                let text = out
                    .value
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if text.trim().is_empty() {
                    return Err(aisetu_core::SetuError::parse_failure(
                        "needle returned empty text",
                    ));
                }
                tracing::debug!(
                    confidence = out.confidence,
                    "used needle fallback for provider extraction"
                );
                let mut response = ConversationResponse::assistant(sanitize(&text));
                response.message = Message::assistant(sanitize(&text));
                validate_response(&response)?;
                Ok(response)
            }
            Err(e) => Err(aisetu_core::SetuError::parse_failure(format!(
                "failed to extract assistant content: {e}"
            ))),
        }
    }
}

fn sanitize(text: &str) -> String {
    text.replace("\u{0000}", "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn extracts_standard_paths() {
        let r = ReliableExtractor::default();
        let rep = ProviderRepresentation::new(
            "x",
            json!({"choices":[{"message":{"content":" hello "}}]}),
        );
        let out = r.extract(&rep).await.unwrap();
        assert_eq!(out.text(), "hello");
    }

    #[tokio::test]
    async fn malformed_null_fails_explicitly() {
        let r = ReliableExtractor::default();
        let err = r
            .extract(&ProviderRepresentation::new("x", Value::Null))
            .await
            .unwrap_err();
        assert_eq!(err.kind, aisetu_core::ErrorKind::ParseFailure);
    }

    #[tokio::test]
    async fn unexpected_shape_uses_needle() {
        let r = ReliableExtractor::default();
        let rep = ProviderRepresentation::new("x", json!({"text": "recovered"}));
        let out = r.extract(&rep).await.unwrap();
        assert_eq!(out.text(), "recovered");
    }

    #[tokio::test]
    async fn missing_content_fails() {
        let r = ReliableExtractor::default();
        let err = r
            .extract(&ProviderRepresentation::new("x", json!({"foo": 1})))
            .await
            .unwrap_err();
        assert_eq!(err.kind, aisetu_core::ErrorKind::ParseFailure);
    }
}
