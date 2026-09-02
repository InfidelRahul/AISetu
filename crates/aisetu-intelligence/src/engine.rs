//! IntelligenceEngine trait and a deterministic implementation for tests.

use async_trait::async_trait;
use serde_json::Value;

use crate::{
    schema::JsonSchema,
    types::{IntelligenceContext, IntelligenceInput, IntelligenceOutput},
};

/// Structured-intelligence contract.
#[async_trait]
pub trait IntelligenceEngine: Send + Sync {
    fn name(&self) -> &'static str;

    async fn infer(
        &self,
        input: &IntelligenceInput,
        schema: &JsonSchema,
        context: &IntelligenceContext,
    ) -> aisetu_core::Result<IntelligenceOutput>;
}

/// Deterministic engine used in tests: parses the input as JSON and validates it.
pub struct DeterministicEngine;

#[async_trait]
impl IntelligenceEngine for DeterministicEngine {
    fn name(&self) -> &'static str {
        "deterministic"
    }

    async fn infer(
        &self,
        input: &IntelligenceInput,
        schema: &JsonSchema,
        _context: &IntelligenceContext,
    ) -> aisetu_core::Result<IntelligenceOutput> {
        let value: Value = serde_json::from_str(input.text.trim()).map_err(|e| {
            aisetu_core::SetuError::parse_failure(format!(
                "deterministic engine expected JSON input: {e}"
            ))
        })?;
        schema.validate(&value)?;
        Ok(IntelligenceOutput::new(value, 1.0, self.name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn deterministic_validates() {
        let engine = DeterministicEngine;
        let schema = JsonSchema::object(&["ok"], vec![("ok", JsonSchema::boolean())]);
        let out = engine
            .infer(
                &IntelligenceInput::new(r#"{"ok": true}"#),
                &schema,
                &IntelligenceContext::default(),
            )
            .await
            .unwrap();
        assert_eq!(out.value, json!({"ok": true}));
        assert_eq!(out.confidence, 1.0);
        assert_eq!(out.engine, "deterministic");
    }

    #[tokio::test]
    async fn deterministic_rejects_invalid() {
        let engine = DeterministicEngine;
        let schema = JsonSchema::object(&["ok"], vec![("ok", JsonSchema::boolean())]);
        let err = engine
            .infer(
                &IntelligenceInput::new(r#"{"ok": "nope"}"#),
                &schema,
                &IntelligenceContext::default(),
            )
            .await
            .unwrap_err();
        assert_eq!(err.kind, aisetu_core::ErrorKind::Validation);
    }
}
