//! Canonical conversation request sent toward a provider adapter.

use serde::{Deserialize, Serialize};

use crate::conversation::Conversation;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationRequest {
    pub conversation: Conversation,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stop: Vec<String>,
    pub stream: bool,
}

impl ConversationRequest {
    pub fn new(conversation: Conversation) -> Self {
        Self {
            conversation,
            model: None,
            temperature: None,
            max_tokens: None,
            stop: Vec::new(),
            stream: false,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn validate(&self) -> aisetu_core::Result<()> {
        self.conversation.validate()?;
        if let Some(t) = self.temperature {
            if !(0.0..=2.0).contains(&t) {
                return Err(aisetu_core::SetuError::validation(format!(
                    "temperature {t} is out of range 0..=2"
                )));
            }
        }
        if let Some(0) = self.max_tokens {
            return Err(aisetu_core::SetuError::validation(
                "max_tokens must be greater than 0",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;

    #[test]
    fn validates_temperature() {
        let mut req =
            ConversationRequest::new(Conversation::with_messages(vec![Message::user("hi")]));
        req.temperature = Some(3.0);
        assert!(req.validate().is_err());
        req.temperature = Some(0.7);
        req.validate().unwrap();
    }
}
