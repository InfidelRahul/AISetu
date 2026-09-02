//! Validate provider representations and extracted conversations.

use aisetu_conversation::{Conversation, ConversationResponse};
use serde_json::Value;

pub trait Validator: Send + Sync {
    fn validate_payload(&self, payload: &Value) -> aisetu_core::Result<()>;
}

/// Require that a JSON object contains the listed keys.
pub struct SchemaValidator {
    pub required_keys: Vec<String>,
}

impl SchemaValidator {
    pub fn new(keys: &[&str]) -> Self {
        Self {
            required_keys: keys.iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

impl Validator for SchemaValidator {
    fn validate_payload(&self, payload: &Value) -> aisetu_core::Result<()> {
        let obj = payload.as_object().ok_or_else(|| {
            aisetu_core::SetuError::validation("provider payload is not a JSON object")
        })?;
        for key in &self.required_keys {
            if !obj.contains_key(key) {
                return Err(aisetu_core::SetuError::validation(format!(
                    "provider payload missing required key '{key}'"
                )));
            }
        }
        Ok(())
    }
}

pub fn validate_response(response: &ConversationResponse) -> aisetu_core::Result<()> {
    response.validate()?;
    if response.text().trim().is_empty() {
        return Err(aisetu_core::SetuError::parse_failure(
            "assistant response is empty after extraction",
        ));
    }
    Ok(())
}

pub fn validate_request_conversation(conversation: &Conversation) -> aisetu_core::Result<()> {
    conversation.validate()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn schema_ok_and_missing() {
        let v = SchemaValidator::new(&["text", "ok"]);
        v.validate_payload(&json!({"text":"a","ok":true})).unwrap();
        assert!(v.validate_payload(&json!({"text":"a"})).is_err());
        assert!(v.validate_payload(&json!("nope")).is_err());
    }
}
