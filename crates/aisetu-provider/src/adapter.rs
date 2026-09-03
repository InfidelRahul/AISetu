//! Provider adapter trait.

use aisetu_conversation::{ConversationRequest, ConversationResponse};
use aisetu_session::Session;
use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::capabilities::ProviderCapabilities;

/// Stable provider identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProviderId(pub String);

impl ProviderId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Provider-specific protocol adapter.
///
/// Conversation Layer → Provider Adapter → HTTP Layer
#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &ProviderId;
    fn capabilities(&self) -> &ProviderCapabilities;

    async fn complete(
        &self,
        request: ConversationRequest,
        session: Option<&Session>,
    ) -> aisetu_core::Result<ConversationResponse>;

    /// Streaming provider output. Providers with native streaming should override this.
    /// The default implementation preserves API compatibility by yielding the completed
    /// response as one delta.
    async fn stream(
        &self,
        request: ConversationRequest,
        session: Option<&Session>,
    ) -> aisetu_core::Result<BoxStream<'static, aisetu_core::Result<String>>> {
        let response = self.complete(request, session).await?;
        let text = response.text().to_string();
        Ok(Box::pin(futures::stream::once(async move { Ok(text) })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_display() {
        let id = ProviderId::new("mock");
        assert_eq!(id.to_string(), "mock");
    }
}
