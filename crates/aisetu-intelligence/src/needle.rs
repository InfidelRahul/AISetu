//! Needle: embedded local intelligence for extracting structured data from text.
//!
//! Needle is a small, deterministic extractor. It does not call remote models.
//! It walks the input looking for JSON objects that satisfy the schema, and
//! falls back to key=value / labeled-line heuristics.

use async_trait::async_trait;
use serde_json::{json, Map, Value};

use crate::{
    engine::IntelligenceEngine,
    schema::{JsonSchema, SchemaType},
    types::{IntelligenceContext, IntelligenceInput, IntelligenceOutput},
};

/// Local structured-intelligence implementation.
pub struct NeedleEngine {
    pub min_confidence: f32,
}

impl Default for NeedleEngine {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
        }
    }
}

impl NeedleEngine {
    pub fn new() -> Self {
        Self::default()
    }

    fn extract(&self, text: &str, schema: &JsonSchema) -> aisetu_core::Result<(Value, f32)> {
        if let Some((value, conf)) = extract_embedded_json(text, schema) {
            return Ok((value, conf));
        }
        if schema.schema_type == SchemaType::Object {
            if let Some((value, conf)) = extract_labeled_fields(text, schema) {
                return Ok((value, conf));
            }
        }
        Err(aisetu_core::SetuError::parse_failure(
            "needle could not extract a value matching the schema",
        ))
    }
}

#[async_trait]
impl IntelligenceEngine for NeedleEngine {
    fn name(&self) -> &'static str {
        "needle"
    }

    async fn infer(
        &self,
        input: &IntelligenceInput,
        schema: &JsonSchema,
        context: &IntelligenceContext,
    ) -> aisetu_core::Result<IntelligenceOutput> {
        let _ = context;
        let (value, confidence) = self.extract(&input.text, schema)?;
        schema.validate(&value)?;
        if confidence < self.min_confidence {
            return Err(aisetu_core::SetuError::validation(format!(
                "needle confidence {confidence:.2} below threshold {:.2}",
                self.min_confidence
            )));
        }
        tracing::debug!(confidence, "needle extracted structured value");
        Ok(IntelligenceOutput::new(value, confidence, self.name()))
    }
}

fn extract_embedded_json(text: &str, schema: &JsonSchema) -> Option<(Value, f32)> {
    let trimmed = text.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if schema.validate(&value).is_ok() {
            return Some((value, 0.99));
        }
    }
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    let slice = &text[start..=end];
    let value = serde_json::from_str::<Value>(slice).ok()?;
    if schema.validate(&value).is_ok() {
        Some((value, 0.9))
    } else {
        None
    }
}

fn extract_labeled_fields(text: &str, schema: &JsonSchema) -> Option<(Value, f32)> {
    let mut map = Map::new();
    let mut hits = 0usize;
    for (key, child) in &schema.properties {
        if let Some(raw) = find_labeled(text, key) {
            if let Some(parsed) = coerce(&raw, child) {
                map.insert(key.clone(), parsed);
                hits += 1;
            }
        }
    }
    if hits == 0 {
        return None;
    }
    let required_hits = schema
        .required
        .iter()
        .filter(|k| map.contains_key(*k))
        .count();
    if required_hits < schema.required.len() {
        return None;
    }
    let confidence = (hits as f32) / (schema.properties.len().max(1) as f32);
    Some((Value::Object(map), 0.5 + 0.4 * confidence))
}

fn find_labeled(text: &str, key: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let key_l = key.to_ascii_lowercase();
    for sep in [":", "=", " is "] {
        let pattern = format!("{key_l}{sep}");
        if let Some(idx) = lower.find(&pattern) {
            let from = idx + pattern.len();
            let rest = text[from..].trim_start();
            let token = rest
                .split(['\n', ',', ';', '}'])
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .to_string();
            if !token.is_empty() {
                return Some(token);
            }
        }
    }
    None
}

fn coerce(raw: &str, schema: &JsonSchema) -> Option<Value> {
    match schema.schema_type {
        SchemaType::String => Some(json!(raw)),
        SchemaType::Number => raw.parse::<f64>().ok().map(|n| json!(n)),
        SchemaType::Boolean => match raw.to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" => Some(json!(true)),
            "false" | "no" | "0" => Some(json!(false)),
            _ => None,
        },
        SchemaType::Object | SchemaType::Array => serde_json::from_str(raw).ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person_schema() -> JsonSchema {
        JsonSchema::object(
            &["name", "age"],
            vec![
                ("name", JsonSchema::string()),
                ("age", JsonSchema::number()),
            ],
        )
    }

    #[tokio::test]
    async fn extracts_embedded_json() {
        let engine = NeedleEngine::new();
        let out = engine
            .infer(
                &IntelligenceInput::new("noise {\"name\":\"Ada\",\"age\":36} trailing"),
                &person_schema(),
                &IntelligenceContext::default(),
            )
            .await
            .unwrap();
        assert_eq!(out.value["name"], "Ada");
        assert_eq!(out.value["age"], 36);
        assert!(out.confidence >= 0.9);
        assert_eq!(out.engine, "needle");
    }

    #[tokio::test]
    async fn extracts_labeled_fields() {
        let engine = NeedleEngine::new();
        let out = engine
            .infer(
                &IntelligenceInput::new("name: Grace\nage: 85"),
                &person_schema(),
                &IntelligenceContext::default(),
            )
            .await
            .unwrap();
        assert_eq!(out.value["name"], "Grace");
        assert_eq!(out.value["age"], 85.0);
        assert!(out.confidence >= 0.5);
    }

    #[tokio::test]
    async fn fails_when_nothing_matches() {
        let engine = NeedleEngine::new();
        let err = engine
            .infer(
                &IntelligenceInput::new("nothing useful here"),
                &person_schema(),
                &IntelligenceContext::default(),
            )
            .await
            .unwrap_err();
        assert_eq!(err.kind, aisetu_core::ErrorKind::ParseFailure);
    }
}
