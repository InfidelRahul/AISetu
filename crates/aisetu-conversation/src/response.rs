//! Canonical conversation response produced by a provider adapter.

use serde::{Deserialize, Serialize};

use crate::message::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl Usage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens.saturating_add(completion_tokens),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationResponse {
    pub message: Message,
    pub finish_reason: FinishReason,
    pub usage: Option<Usage>,
    pub model: Option<String>,
    pub provider: Option<String>,
}

impl ConversationResponse {
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            message: Message::assistant(text),
            finish_reason: FinishReason::Stop,
            usage: None,
            model: None,
            provider: None,
        }
    }

    pub fn text(&self) -> &str {
        self.message.content.as_str()
    }

    pub fn validate(&self) -> aisetu_core::Result<()> {
        if self.message.role != crate::Role::Assistant {
            return Err(aisetu_core::SetuError::validation(
                "conversation response message must have assistant role",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assistant_response() {
        let r = ConversationResponse::assistant("ok");
        r.validate().unwrap();
        assert_eq!(r.text(), "ok");
        let json = serde_json::to_string(&r).unwrap();
        let back: ConversationResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }
}
