//! Second provider: echoes the last user message.
//!
//! Adding this provider requires only an adapter + configuration, not API changes.

use aisetu_conversation::{ConversationRequest, ConversationResponse, Usage};
use aisetu_session::Session;
use async_trait::async_trait;

use crate::{
    adapter::{Provider, ProviderId},
    capabilities::ProviderCapabilities,
};

pub struct EchoProvider {
    id: ProviderId,
    capabilities: ProviderCapabilities,
}

impl EchoProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ProviderId::new(name),
            capabilities: ProviderCapabilities::text_only().with_streaming(),
        }
    }
}

#[async_trait]
impl Provider for EchoProvider {
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
        let last = request
            .conversation
            .last_user()
            .ok_or_else(|| aisetu_core::SetuError::invalid_request("no user message"))?;
        let mut response = ConversationResponse::assistant(last.content.as_str());
        response.provider = Some(self.id.as_str().to_string());
        response.model = request.model.clone();
        response.usage = Some(Usage::new(
            last.content.as_str().len() as u32 / 4,
            last.content.as_str().len() as u32 / 4,
        ));
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aisetu_conversation::{Conversation, Message};

    #[tokio::test]
    async fn echoes() {
        let p = EchoProvider::new("echo");
        let req = ConversationRequest::new(Conversation::with_messages(vec![
            Message::system("sys"),
            Message::user("ping"),
        ]));
        let resp = p.complete(req, None).await.unwrap();
        assert_eq!(resp.text(), "ping");
        assert_eq!(resp.provider.as_deref(), Some("echo"));
    }
}
