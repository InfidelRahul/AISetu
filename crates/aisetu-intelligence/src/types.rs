//! Structured intelligence contract types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Input document presented to the intelligence engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntelligenceInput {
    pub text: String,
}

impl IntelligenceInput {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

/// Optional surrounding context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct IntelligenceContext {
    pub source: Option<String>,
    pub hints: Vec<String>,
}

/// Validated structured result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntelligenceOutput {
    pub value: Value,
    pub confidence: f32,
    pub engine: String,
}

impl IntelligenceOutput {
    pub fn new(value: Value, confidence: f32, engine: impl Into<String>) -> Self {
        Self {
            value,
            confidence: confidence.clamp(0.0, 1.0),
            engine: engine.into(),
        }
    }
}
