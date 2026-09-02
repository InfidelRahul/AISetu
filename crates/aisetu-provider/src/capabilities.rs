//! Formal provider capability descriptions.

use serde::{Deserialize, Serialize};

/// Individual capability a provider may advertise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Text,
    Streaming,
    SystemMessages,
    JsonMode,
}

/// Capability set attached to a provider/model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub text: bool,
    pub streaming: bool,
    pub system_messages: bool,
    pub json_mode: bool,
    pub max_context_tokens: u32,
    pub max_output_tokens: u32,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            text: true,
            streaming: false,
            system_messages: true,
            json_mode: false,
            max_context_tokens: 8_192,
            max_output_tokens: 2_048,
        }
    }
}

impl ProviderCapabilities {
    pub fn text_only() -> Self {
        Self::default()
    }

    pub fn with_streaming(mut self) -> Self {
        self.streaming = true;
        self
    }

    pub fn supports(&self, cap: Capability) -> bool {
        match cap {
            Capability::Text => self.text,
            Capability::Streaming => self.streaming,
            Capability::SystemMessages => self.system_messages,
            Capability::JsonMode => self.json_mode,
        }
    }

    pub fn require(&self, cap: Capability) -> aisetu_core::Result<()> {
        if self.supports(cap) {
            Ok(())
        } else {
            Err(aisetu_core::SetuError::invalid_request(format!(
                "provider does not support capability '{cap:?}'"
            )))
        }
    }

    pub fn fits_context(&self, estimated_tokens: u32) -> bool {
        estimated_tokens <= self.max_context_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_streaming() {
        let caps = ProviderCapabilities::text_only();
        assert!(caps.require(Capability::Text).is_ok());
        assert!(caps.require(Capability::Streaming).is_err());
        let caps = caps.with_streaming();
        assert!(caps.require(Capability::Streaming).is_ok());
    }
}
