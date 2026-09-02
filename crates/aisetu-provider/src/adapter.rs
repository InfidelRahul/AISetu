//! Provider adapter trait.

use aisetu_conversation::{ConversationRequest, ConversationResponse};
use aisetu_session::Session;
use async_trait::async_trait;

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
