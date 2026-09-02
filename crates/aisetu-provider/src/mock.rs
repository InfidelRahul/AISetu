//! In-process mock provider used as a development default.

use aisetu_conversation::{ConversationRequest, ConversationResponse, Usage};
use aisetu_session::Session;
use async_trait::async_trait;

use crate::{
    adapter::{Provider, ProviderId},
    capabilities::ProviderCapabilities,
};

pub struct MockProvider {
    id: ProviderId,
    capabilities: ProviderCapabilities,
    reply_prefix: String,
}

impl MockProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ProviderId::new(name),
            capabilities: ProviderCapabilities::text_only().with_streaming(),
            reply_prefix: "mock:".into(),
        }
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.reply_prefix = prefix.into();
        self
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn id(&self) -> &ProviderId {
        &self.id
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn complete(
        &self,
        request: ConversationRequest,
        _session: Option<&Session>,
    ) -> aisetu_core::Result<ConversationResponse> {
        request.validate()?;
        self.capabilities
            .require(crate::capabilities::Capability::Text)?;
        let last = request
            .conversation
            .last_user()
            .ok_or_else(|| aisetu_core::SetuError::invalid_request("no user message"))?;
        let text = format!("{} {}", self.reply_prefix, last.content.as_str());
        let mut response = ConversationResponse::assistant(text);
        response.provider = Some(self.id.as_str().to_string());
        response.model = request.model.clone();
        response.usage = Some(Usage::new(request.conversation.len() as u32 * 8, 8));
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aisetu_conversation::{Conversation, Message};

    #[tokio::test]
    async fn replies_to_user() {
        let p = MockProvider::new("mock");
        let req =
            ConversationRequest::new(Conversation::with_messages(vec![Message::user("hello")]))
                .with_model("mock-text");
        let resp = p.complete(req, None).await.unwrap();
        assert!(resp.text().contains("hello"));
        assert_eq!(resp.provider.as_deref(), Some("mock"));
    }
}
